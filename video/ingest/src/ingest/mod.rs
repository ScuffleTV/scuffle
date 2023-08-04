use anyhow::Result;
use common::prelude::FutureTimeout;
use std::{sync::Arc, time::Duration};
use tokio::{net::TcpSocket, select};

use crate::global::GlobalState;

mod bytes_tracker;
mod connection;
mod errors;
mod rtmp_session;
mod update;

pub async fn run(global: Arc<GlobalState>) -> Result<()> {
    tracing::info!("Listening on {}", global.config.rtmp.bind_address);
    let socket = if global.config.rtmp.bind_address.is_ipv6() {
        TcpSocket::new_v6()?
    } else {
        TcpSocket::new_v4()?
    };

    socket.set_reuseaddr(true)?;
    socket.set_reuseport(true)?;
    socket.bind(global.config.rtmp.bind_address)?;
    let listener = socket.listen(1024)?;
    let tls_acceptor = if let Some(tls) = &global.config.rtmp.tls {
        tracing::info!("TLS enabled");
        let cert = std::fs::read(&tls.cert).expect("failed to read rtmp cert");
        let key = std::fs::read(&tls.key).expect("failed to read rtmp key");

        Some(Arc::new(tokio_native_tls::TlsAcceptor::from(
            native_tls::TlsAcceptor::new(native_tls::Identity::from_pkcs8(&cert, &key)?)?,
        )))
    } else {
        None
    };

    loop {
        select! {
            _ = global.ctx.done() => {
                return Ok(());
            },
            r = listener.accept() => {
                let (socket, addr) = r?;
                tracing::debug!("Accepted connection from {}", addr);

                let tls_acceptor = tls_acceptor.clone();
                let global = global.clone();

                tokio::spawn(async move {
                    if let Some(tls_acceptor) = tls_acceptor {
                        let Ok(Ok(socket)) = tls_acceptor.accept(socket).timeout(Duration::from_secs(5)).await else {
                            return;
                        };

                        tracing::debug!("TLS handshake complete");
                        connection::handle(global, socket, addr.ip()).await;
                    } else {
                        connection::handle(global, socket, addr.ip()).await;
                    }
                });
            },
        }
    }
}
