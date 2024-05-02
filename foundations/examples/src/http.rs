use std::convert::Infallible;
use std::net::{SocketAddr, TcpListener as StdTcpListener};

use hyper::body::{Bytes, Incoming};
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use opentelemetry::trace::Status;
use rand::Rng;
use scuffle_foundations::bootstrap::{bootstrap, Bootstrap, RuntimeSettings};
use scuffle_foundations::runtime::spawn;
use scuffle_foundations::settings::auto_settings;
use scuffle_foundations::settings::cli::Matches;
use scuffle_foundations::telemetry::opentelemetry::OpenTelemetrySpanExt;
use scuffle_foundations::telemetry::settings::TelemetrySettings;
use scuffle_foundations::{wrapped, BootstrapResult};
use socket2::Socket;
use tokio::net::{TcpListener, TcpStream};

type Body = http_body_util::Full<Bytes>;

#[auto_settings]
#[serde(default)]
struct Config {
	telemetry: TelemetrySettings,
	runtime: RuntimeSettings,
	#[settings(default = SocketAddr::from(([127, 0, 0, 1], 8080)))]
	bind: SocketAddr,
	#[settings(default = 1)]
	listener_count: usize,
}

impl Bootstrap for Config {
	type Settings = Self;

	fn runtime_mode(&self) -> RuntimeSettings {
		self.runtime.clone()
	}

	fn telemetry_config(&self) -> Option<TelemetrySettings> {
		Some(self.telemetry.clone())
	}
}

fn create_listner(bind: SocketAddr) -> BootstrapResult<StdTcpListener> {
	let listener = Socket::new(socket2::Domain::IPV4, socket2::Type::STREAM, Some(socket2::Protocol::TCP))?;

	listener.set_reuse_address(true)?;
	listener.set_reuse_port(true)?;
	listener.set_keepalive(true)?;

	listener.bind(&bind.into())?;
	listener.set_nonblocking(true)?;
	listener.listen(1024)?;

	Ok(listener.into())
}

#[bootstrap]
async fn main(cli: Matches<Config>) -> BootstrapResult<()> {
	tracing::info!("starting");

	for i in 0..cli.settings.listener_count {
		let listener = create_listner(cli.settings.bind)?;

		tracing::debug!(idx = %i, "starting listener");
		spawn(run_endpoint(i, listener));
	}

	tracing::info!("started");

	scuffle_foundations::signal::SignalHandler::new()
		.with_signal(tokio::signal::unix::SignalKind::interrupt())
		.with_signal(tokio::signal::unix::SignalKind::terminate())
		.recv()
		.await;

	tracing::info!("stopping");

	scuffle_foundations::context::Handler::global().shutdown().await;

	tracing::info!("stopped");

	Ok(())
}

#[tracing::instrument(skip(listener))]
async fn run_endpoint(idx: usize, listener: StdTcpListener) -> BootstrapResult<()> {
	let listener = TcpListener::from_std(listener)?;

	tracing::info!("listening");

	loop {
		match listener.accept().await {
			Ok((conn, client_addr)) => {
				spawn(serve_connection(conn, client_addr));
			}
			Err(e) => {
				tracing::error!(err = %e, "failed to accept connection");
			}
		}
	}
}

#[tracing::instrument(skip(conn))]
async fn serve_connection(conn: TcpStream, _: SocketAddr) {
	tracing::trace!("accepted client connection");

	let on_request = service_fn(respond);

	let mut http = hyper::server::conn::http1::Builder::new();

	http.keep_alive(true);

	http.serve_connection(TokioIo::new(conn), on_request).await.ok();

	tracing::trace!("closed client connection");
}

#[wrapped(map_response)]
#[tracing::instrument(skip(req), fields(path = req.uri().path(), method = req.method().as_str(), response.status))]
async fn respond(req: Request<Incoming>) -> Result<Response<Body>, Infallible> {
	tracing::Span::current().make_root();
	tracing::trace!("received request");

	let response = match req.uri().path() {
		"/hello" => hello_req(req).await?,
		_ => {
			let body = Bytes::from_static(b"Not Found");
			Response::builder()
				.status(404)
				.header("Content-Type", "text/plain")
				.body(body.into())
				.unwrap()
		}
	};

	Ok(response)
}

fn map_response(result: Result<Response<Body>, Infallible>) -> Result<Response<Body>, Infallible> {
	let span = tracing::Span::current();
	tracing::debug!("where am i?");

	result
		.map(|mut ok| {
			span.record("response.status", ok.status().as_u16());
			span.set_status(Status::Ok);

			span.trace_id().map(|trace_id| {
				ok.headers_mut().insert("X-Ray-Id", trace_id.to_string().parse().unwrap());
			});

			ok
		})
		.inspect_err(|err| {
			span.record("response.status", 500);
			span.set_status(Status::Error {
				description: err.to_string().into(),
			});
		})
}

#[wrapped(map_response_resource)]
#[tracing::instrument]
async fn load_resource() -> Result<(), &'static str> {
	tokio::time::sleep(std::time::Duration::from_millis(30)).await;
	if rand::thread_rng().gen_bool(0.01) {
		Err("failed to load resource")
	} else {
		Ok(())
	}
}

fn map_response_resource(result: Result<(), &'static str>) -> Result<(), &'static str> {
	let span = tracing::Span::current();

	result
		.inspect(|_| {
			span.set_status(Status::Ok);
		})
		.inspect_err(|err| {
			span.set_status(Status::Error {
				description: err.to_string().into(),
			});
		})
}

#[tracing::instrument]
async fn hello_req(_: Request<Incoming>) -> Result<Response<Body>, Infallible> {
	let body = Bytes::from_static(b"Hello, World!");

	if let Err(err) = load_resource().await {
		Ok(Response::builder()
			.status(500)
			.header("Content-Type", "text/plain")
			.body(Bytes::from(err).into())
			.unwrap())
	} else {
		Ok(Response::builder()
			.status(200)
			.header("Content-Type", "text/plain")
			.body(body.into())
			.unwrap())
	}
}
