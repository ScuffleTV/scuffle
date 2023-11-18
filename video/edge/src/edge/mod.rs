use std::io;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use common::http::RouteError;
use common::prelude::FutureTimeout;
use hyper::http::header;
use hyper::server::conn::Http;
use hyper::Body;
use routerify::{Middleware, RequestServiceBuilder, Router};
use tokio::net::TcpSocket;
use tokio::select;

use crate::config::EdgeConfig;
use crate::global::EdgeGlobal;

mod error;
mod stream;

pub use error::EdgeError;

pub fn cors_middleware<G: EdgeGlobal>(_: &Arc<G>) -> Middleware<Body, RouteError<EdgeError>> {
	Middleware::post(|mut resp| async move {
		resp.headers_mut()
			.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
		resp.headers_mut()
			.insert(header::ACCESS_CONTROL_ALLOW_METHODS, "*".parse().unwrap());
		resp.headers_mut()
			.insert(header::ACCESS_CONTROL_ALLOW_HEADERS, "*".parse().unwrap());
		resp.headers_mut()
			.insert(header::ACCESS_CONTROL_EXPOSE_HEADERS, "Date".parse().unwrap());
		resp.headers_mut().insert("Timing-Allow-Origin", "*".parse().unwrap());
		resp.headers_mut().insert(
			header::ACCESS_CONTROL_MAX_AGE,
			Duration::from_secs(86400).as_secs().to_string().parse().unwrap(),
		);

		Ok(resp)
	})
}

pub fn routes<G: EdgeGlobal>(global: &Arc<G>) -> Router<Body, RouteError<EdgeError>> {
	let weak = Arc::downgrade(global);
	Router::builder()
		.data(weak)
		.err_handler_with_info(common::http::error_handler::<EdgeError>)
		.middleware(cors_middleware(global))
		.scope("/", stream::routes(global))
		.build()
		.expect("failed to build router")
}

pub async fn run<G: EdgeGlobal>(global: Arc<G>) -> anyhow::Result<()> {
	let config = global.config::<EdgeConfig>();
	tracing::info!("Edge(HTTP) listening on {}", config.bind_address);
	let socket = if config.bind_address.is_ipv6() {
		TcpSocket::new_v6()?
	} else {
		TcpSocket::new_v4()?
	};

	socket.set_reuseaddr(true)?;
	socket.set_reuseport(true)?;
	socket.bind(config.bind_address)?;
	let listener = socket.listen(1024)?;

	let tls_acceptor = if let Some(tls) = &config.tls {
		tracing::info!("TLS enabled");
		let cert = tokio::fs::read(&tls.cert).await.context("failed to read edge ssl cert")?;
		let key = tokio::fs::read(&tls.key)
			.await
			.context("failed to read edge ssl private key")?;

		let key =
			rustls::PrivateKey(rustls_pemfile::pkcs8_private_keys(&mut io::BufReader::new(io::Cursor::new(key)))?.remove(0));

		let certs = rustls_pemfile::certs(&mut io::BufReader::new(io::Cursor::new(cert)))?
			.into_iter()
			.map(rustls::Certificate)
			.collect();

		Some(Arc::new(tokio_rustls::TlsAcceptor::from(Arc::new(
			rustls::ServerConfig::builder()
				.with_safe_defaults()
				.with_no_client_auth()
				.with_single_cert(certs, key)?,
		))))
	} else {
		None
	};

	// The reason we use a Weak reference to the global state is because we don't
	// want to block the shutdown When a keep-alive connection is open, the request
	// service will still be alive, and will still be holding a reference to the
	// global state If we used an Arc, the global state would never be dropped, and
	// the shutdown would never complete By using a Weak reference, we can check if
	// the global state is still alive, and if it isn't, we can stop accepting new
	// connections
	let request_service = RequestServiceBuilder::new(routes(&global)).expect("failed to build request service");

	loop {
		select! {
			_ = global.ctx().done() => {
				return Ok(());
			},
			r = listener.accept() => {
				let (socket, addr) = r?;

				let tls_acceptor = tls_acceptor.clone();
				let service = request_service.build(addr);

				tracing::debug!("Accepted connection from {}", addr);

				tokio::spawn(async move {
					 if let Some(tls_acceptor) = tls_acceptor {
						let Ok(Ok(socket)) = tls_acceptor.accept(socket).timeout(Duration::from_secs(5)).await else {
							return;
						};
						tracing::debug!("TLS handshake complete");
						Http::new().serve_connection(
							socket,
							service,
						).with_upgrades().await.ok();
					} else {
						 Http::new().serve_connection(
							socket,
							service,
						).with_upgrades().await.ok();
					}
				});
			},
		}
	}
}
