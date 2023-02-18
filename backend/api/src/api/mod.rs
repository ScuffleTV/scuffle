use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use hyper::{service::service_fn, Body, Request, Response, StatusCode};
use tokio::net::TcpListener;
use tracing::instrument;

use crate::global::GlobalState;

#[instrument(name = "hello_world", skip(req), fields(method = req.method().to_string(), path = &req.uri().path()))]
async fn hello_world(req: Request<Body>) -> Result<Response<Body>> {
    tracing::debug!("Hii there!");

    Ok(Response::new("Hello, World".into()))
}

pub async fn run(config: Arc<GlobalState>) -> Result<()> {
    let addr: SocketAddr = config.config.bind_address.parse()?;

    tracing::info!("Listening on {}", addr);
    let listener = TcpListener::bind(&addr).await?;

    loop {
        let (socket, _) = listener.accept().await?;

        tracing::debug!("Accepted connection from {}", socket.peer_addr()?);

        let conn = hyper::server::conn::Http::new().serve_connection(
            socket,
            service_fn(|req| async {
                match req.uri().path() {
                    "/hello" => hello_world(req).await,
                    _ => Ok(Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body("Not Found".into())?),
                }
            }),
        );

        tokio::spawn(conn);
    }
}
