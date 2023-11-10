use std::io;
use std::{sync::Arc, time::Duration};

use anyhow::Result;
use common::prelude::FutureTimeout;
use hyper::http::header;
use hyper::{server::conn::Http, Body, Response, StatusCode};
use routerify::{Middleware, RequestInfo, RequestServiceBuilder, Router};
use serde_json::json;
use tokio::net::TcpSocket;
use tokio::select;

use crate::{edge::macros::make_response, global::GlobalState};

use self::error::{RouteError, ShouldLog};

mod error;
mod ext;
mod macros;
mod stream;

async fn error_handler(
    err: Box<(dyn std::error::Error + Send + Sync + 'static)>,
    info: RequestInfo,
) -> Response<Body> {
    match err.downcast::<RouteError>() {
        Ok(err) => {
            let location = err.location();

            err.span().in_scope(|| match err.should_log() {
                ShouldLog::Yes => {
                    tracing::error!(location = location.to_string(), error = ?err, "http error")
                }
                ShouldLog::Debug => {
                    tracing::debug!(location = location.to_string(), error = ?err, "http error")
                }
                ShouldLog::No => (),
            });

            err.response()
        }
        Err(err) => {
            tracing::error!(error = ?err, info = ?info, "unhandled http error");
            make_response!(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "message": "Internal Server Error", "success": false })
            )
        }
    }
}

pub fn cors_middleware(_: &Arc<GlobalState>) -> Middleware<Body, RouteError> {
    Middleware::post(|mut resp| async move {
        resp.headers_mut()
            .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
        resp.headers_mut()
            .insert(header::ACCESS_CONTROL_ALLOW_METHODS, "*".parse().unwrap());
        resp.headers_mut()
            .insert(header::ACCESS_CONTROL_ALLOW_HEADERS, "*".parse().unwrap());
        resp.headers_mut().insert(
            header::ACCESS_CONTROL_EXPOSE_HEADERS,
            "Date".parse().unwrap(),
        );
        resp.headers_mut()
            .insert("Timing-Allow-Origin", "*".parse().unwrap());
        resp.headers_mut().insert(
            header::ACCESS_CONTROL_MAX_AGE,
            Duration::from_secs(86400)
                .as_secs()
                .to_string()
                .parse()
                .unwrap(),
        );

        Ok(resp)
    })
}

pub fn routes(global: &Arc<GlobalState>) -> Router<Body, RouteError> {
    let weak = Arc::downgrade(global);
    Router::builder()
        .data(weak)
        // Our error handler
        .err_handler_with_info(error_handler)
        .middleware(cors_middleware(global))
        .scope("/", stream::routes(global))
        .build()
        .expect("failed to build router")
}

pub async fn run(global: Arc<GlobalState>) -> Result<()> {
    tracing::info!("Listening on {}", global.config.edge.bind_address);
    let socket = if global.config.edge.bind_address.is_ipv6() {
        TcpSocket::new_v6()?
    } else {
        TcpSocket::new_v4()?
    };

    socket.set_reuseaddr(true)?;
    socket.set_reuseport(true)?;
    socket.bind(global.config.edge.bind_address)?;
    let listener = socket.listen(1024)?;

    let tls_acceptor = if let Some(tls) = &global.config.edge.tls {
        tracing::info!("TLS enabled");
        let cert = std::fs::read(&tls.cert).expect("failed to read rtmp cert");
        let key = std::fs::read(&tls.key).expect("failed to read rtmp key");

        let key = rustls::PrivateKey(
            rustls_pemfile::pkcs8_private_keys(&mut io::BufReader::new(io::Cursor::new(key)))?
                .remove(0),
        );

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

    // The reason we use a Weak reference to the global state is because we don't want to block the shutdown
    // When a keep-alive connection is open, the request service will still be alive, and will still be holding a reference to the global state
    // If we used an Arc, the global state would never be dropped, and the shutdown would never complete
    // By using a Weak reference, we can check if the global state is still alive, and if it isn't, we can stop accepting new connections
    let request_service =
        RequestServiceBuilder::new(routes(&global)).expect("failed to build request service");

    loop {
        select! {
            _ = global.ctx.done() => {
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
