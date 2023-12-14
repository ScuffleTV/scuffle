use std::io;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use common::http::RouteError;
use common::prelude::FutureTimeout;
use hyper::server::conn::Http;
use hyper::Body;
use routerify::{RequestServiceBuilder, Router};
use tokio::net::TcpSocket;
use tokio::select;

use self::error::ApiError;
use crate::config::ApiConfig;
use crate::global::ApiGlobal;

mod auth;
mod error;
mod jwt;
mod middleware;
mod request_context;
pub mod v1;

pub fn routes<G: ApiGlobal>(global: &Arc<G>) -> Router<Body, RouteError<ApiError>> {
	let weak = Arc::downgrade(global);
	Router::builder()
		.data(weak)
		// These response header middlewares lets us add headers to the response from the request
		// handlers
		.middleware(middleware::response_headers::pre_flight_middleware(global))
		.middleware(middleware::response_headers::post_flight_middleware(global))
		// Our error handler
		.err_handler_with_info(common::http::error_handler::<ApiError>)
		// The CORS middleware adds the CORS headers to the response
		.middleware(middleware::cors::cors_middleware(global))
		// The auth middleware checks the Authorization header, and if it's valid, it adds the user
		// to the request extensions This way, we can access the user in the handlers, this does not
		// fail the request if the token is invalid or not present.
		.middleware(middleware::auth::auth_middleware(global))
		.scope("/v1", v1::routes(global))
		.build()
		.expect("failed to build router")
}

pub async fn run<G: ApiGlobal>(global: Arc<G>) -> anyhow::Result<()> {
	let config = global.config::<ApiConfig>();

	tracing::info!("Listening on {}", config.bind_address);
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
		let cert = tokio::fs::read(&tls.cert).await.context("failed to read platform ssl cert")?;
		let key = tokio::fs::read(&tls.key)
			.await
			.context("failed to read platform ssl private key")?;

		let key = rustls_pemfile::pkcs8_private_keys(&mut io::BufReader::new(io::Cursor::new(key)))
			.next()
			.ok_or_else(|| anyhow::anyhow!("failed to find private key in platform private key file"))??
			.into();

		let certs = rustls_pemfile::certs(&mut io::BufReader::new(io::Cursor::new(cert))).collect::<Result<Vec<_>, _>>()?;

		Some(Arc::new(tokio_rustls::TlsAcceptor::from(Arc::new(
			rustls::ServerConfig::builder()
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
