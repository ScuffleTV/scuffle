use anyhow::Result;
use hyper::{server::conn::Http, Body};
use routerify::{RequestServiceBuilder, Router};
use std::{convert::Infallible, net::SocketAddr, sync::Arc};
use tokio::{net::TcpSocket, select};

use crate::global::GlobalState;

mod v1;

pub fn routes(global: Arc<GlobalState>) -> Router<Body, Infallible> {
    Router::builder()
        .data(global)
        .scope("/v1", v1::routes())
        .build()
        .unwrap()
}

pub async fn run(global: Arc<GlobalState>) -> Result<()> {
    let addr: SocketAddr = global.config.bind_address.parse()?;

    tracing::info!("Listening on {}", addr);
    let socket = if addr.is_ipv6() {
        TcpSocket::new_v6()?
    } else {
        TcpSocket::new_v4()?
    };
    socket.set_reuseaddr(true)?;
    socket.set_reuseport(true)?;
    socket.bind(addr)?;
    let listener = socket.listen(1024)?;

    let request_service = RequestServiceBuilder::new(routes(global.clone()))
        .expect("failed to build request service");

    loop {
        select! {
            _ = global.ctx.done() => {
                tracing::info!("Shutting down");
                return Ok(());
            },
            r = listener.accept() => {
                let (socket, addr) = r?;
                tracing::debug!("Accepted connection from {}", addr);

                tokio::spawn(Http::new().serve_connection(
                    socket,
                    request_service.build(addr),
                ));
            },
        }
    }
}

#[cfg(test)]
mod tests;
