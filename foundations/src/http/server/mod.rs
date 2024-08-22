use std::net::SocketAddr;
use std::sync::Arc;

mod builder;
pub mod stream;

pub use axum;
use hyper_util::rt::TokioExecutor;
#[cfg(not(feature = "runtime"))]
use tokio::spawn;
#[cfg(feature = "tracing")]
use tracing::Instrument;

pub use self::builder::ServerBuilder;
#[cfg(feature = "http3")]
use self::stream::quic::QuicBackend;
use self::stream::tcp::TcpBackend;
#[cfg(feature = "http-tls")]
use self::stream::tls::TlsBackend;
use self::stream::{Backend, MakeService};
#[cfg(feature = "runtime")]
use crate::runtime::spawn;

#[cfg(feature = "http3")]
#[derive(Clone)]
struct Quic {
	h3: Arc<h3::server::Builder>,
	config: quinn::ServerConfig,
}

#[cfg(feature = "http3")]
impl std::fmt::Debug for Quic {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Quic").finish()
	}
}

pub struct Server<M> {
	bind: SocketAddr,
	#[cfg(feature = "http-tls")]
	insecure_bind: Option<SocketAddr>,
	#[cfg(feature = "http-tls")]
	tls: Option<Arc<rustls::ServerConfig>>,
	http1_2: Arc<hyper_util::server::conn::auto::Builder<TokioExecutor>>,
	#[cfg(feature = "http3")]
	quic: Option<Quic>,
	make_service: M,
	backends: Vec<AbortOnDrop>,
	handler: Option<crate::context::Handler>,
	worker_count: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("io: {0}")]
	Io(#[from] std::io::Error),
	#[error("no bind address specified")]
	NoBindAddress,
	#[cfg(feature = "http3")]
	#[error("quinn connection: {0}")]
	QuinnConnection(#[from] quinn::ConnectionError),
	#[cfg(feature = "http3")]
	#[error("http3: {0}")]
	Http3(#[from] h3::Error),
	#[error("connection closed")]
	ConnectionClosed,
	#[error("axum: {0}")]
	Axum(#[from] axum::Error),
	#[error("{0}")]
	Other(#[from] Box<dyn std::error::Error + Send + Sync>),
	#[error("task join: {0}")]
	TaskJoin(#[from] tokio::task::JoinError),
}

impl Server<()> {
	pub fn builder() -> ServerBuilder {
		ServerBuilder::default()
	}
}

struct AbortOnDrop(Option<tokio::task::JoinHandle<Result<crate::context::Handler, Error>>>);

impl AbortOnDrop {
	fn new(inner: tokio::task::JoinHandle<Result<crate::context::Handler, Error>>) -> Self {
		Self(Some(inner))
	}

	fn into_inner(mut self) -> tokio::task::JoinHandle<Result<crate::context::Handler, Error>> {
		let inner = self.0.take();
		inner.expect("inner task handle already taken")
	}
}

impl Drop for AbortOnDrop {
	fn drop(&mut self) {
		if let Some(inner) = self.0.take() {
			inner.abort();
		}
	}
}

fn ip_mode(addr: SocketAddr) -> std::io::Result<socket2::Domain> {
	if addr.ip().is_ipv4() {
		Ok(socket2::Domain::IPV4)
	} else if addr.ip().is_ipv6() {
		Ok(socket2::Domain::IPV6)
	} else {
		Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid ip address"))
	}
}

fn make_tcp_listener(addr: SocketAddr) -> std::io::Result<tokio::net::TcpListener> {
	let socket = socket2::Socket::new(ip_mode(addr)?, socket2::Type::STREAM, Some(socket2::Protocol::TCP))?;

	socket.set_nonblocking(true)?;
	socket.set_reuse_address(true)?;
	socket.set_reuse_port(true)?;
	socket.bind(&socket2::SockAddr::from(addr))?;
	socket.listen(1024)?;
	socket.set_only_v6(false)?;

	tokio::net::TcpListener::from_std(socket.into())
}

#[cfg(feature = "http3")]
fn make_udp_socket(addr: SocketAddr) -> std::io::Result<std::net::UdpSocket> {
	let socket = socket2::Socket::new(ip_mode(addr)?, socket2::Type::DGRAM, Some(socket2::Protocol::UDP))?;

	socket.set_nonblocking(true)?;
	socket.set_reuse_address(true)?;
	socket.set_reuse_port(true)?;
	socket.bind(&socket2::SockAddr::from(addr))?;
	socket.set_only_v6(false)?;

	Ok(socket.into())
}

impl<M: MakeService> Server<M> {
	pub async fn start(&mut self) -> Result<(), Error> {
		self.backends.clear();
		if let Some(handler) = self.handler.take() {
			handler.cancel();
		}

		let ctx = {
			let (ctx, handler) = crate::context::Context::new();
			self.handler = Some(handler);
			ctx
		};

		#[cfg(feature = "http-tls")]
		if let Some(tls) = self.tls.clone() {
			let acceptor = Arc::new(tokio_rustls::TlsAcceptor::from(tls));
			for i in 0..self.worker_count {
				let tcp_listener = make_tcp_listener(self.bind)?;
				let make_service = self.make_service.clone();
				let backend = TlsBackend::new(tcp_listener, acceptor.clone(), self.http1_2.clone(), &ctx);
				let span = tracing::info_span!("tls", addr = %self.bind, worker = i);
				self.backends
					.push(AbortOnDrop::new(spawn(backend.serve(make_service).instrument(span))));
			}
		} else if self.insecure_bind.is_none() {
			self.insecure_bind = Some(self.bind);
		}

		#[cfg(feature = "http-tls")]
		let bind = self.insecure_bind;
		#[cfg(not(feature = "http-tls"))]
		let bind = Some(self.bind);

		if let Some(addr) = bind {
			for i in 0..self.worker_count {
				let tcp_listener = make_tcp_listener(addr)?;
				let make_service = self.make_service.clone();
				let backend = TcpBackend::new(tcp_listener, self.http1_2.clone(), &ctx);
				let span = tracing::info_span!("tcp", addr = %addr, worker = i);
				self.backends
					.push(AbortOnDrop::new(spawn(backend.serve(make_service).instrument(span))));
			}
		}

		#[cfg(feature = "http3")]
		if let Some(quic) = &self.quic {
			for i in 0..self.worker_count {
				let socket = make_udp_socket(self.bind)?;
				let endpoint = quinn::Endpoint::new(
					quinn::EndpointConfig::default(),
					Some(quic.config.clone()),
					socket,
					quinn::default_runtime().unwrap(),
				)?;
				let make_service = self.make_service.clone();
				let backend = QuicBackend::new(endpoint, quic.h3.clone(), &ctx);
				let span = tracing::info_span!("quic", addr = %self.bind, worker = i);
				self.backends
					.push(AbortOnDrop::new(spawn(backend.serve(make_service).instrument(span))));
			}
		}

		let mut binds = vec![];

		#[cfg(feature = "http-tls")]
		if let Some(insecure_bind) = self.insecure_bind {
			binds.push(format!("http://{insecure_bind}"));
		}
		#[cfg(not(feature = "http-tls"))]
		binds.push(format!("http://{bind}", bind = self.bind));

		#[cfg(feature = "http-tls")]
		if self.tls.is_some() {
			binds.push(format!("https://{}", self.bind));
		}

		#[cfg(feature = "http3")]
		if self.quic.is_some() {
			binds.push(format!("https+quic://{}", self.bind));
		}

		tracing::info!(
			worker_count = self.worker_count,
			"listening on {binds}",
			binds = binds.join(", ")
		);

		Ok(())
	}

	pub async fn start_and_wait(&mut self) -> Result<(), Error> {
		self.start().await?;
		self.wait().await
	}

	pub async fn wait(&mut self) -> Result<(), Error> {
		let Some(handler) = &self.handler else {
			return Ok(());
		};

		let result = futures::future::try_join_all(self.backends.iter_mut().map(|backend| async move {
			let child_handler = backend.0.as_mut().unwrap().await??;
			handler.cancel();
			child_handler.shutdown().await;
			Ok::<_, Error>(())
		}))
		.await;

		self.backends.clear();

		handler.cancel();

		let handler = self.handler.take().unwrap();

		result?;

		handler.shutdown().await;

		Ok(())
	}

	pub async fn shutdown(&mut self) -> Result<(), Error> {
		let Some(handler) = self.handler.take() else {
			return Ok(());
		};

		handler.cancel();

		futures::future::try_join_all(self.backends.drain(..).map(|backend| async move {
			let child_handler = backend.into_inner().await??;
			child_handler.shutdown().await;
			Ok::<_, Error>(())
		}))
		.await?;

		handler.shutdown().await;

		Ok(())
	}
}
