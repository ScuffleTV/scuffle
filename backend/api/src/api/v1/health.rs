use std::convert::Infallible;

use hyper::{Body, Request, Response, StatusCode};
use routerify::Router;

async fn health(_: Request<Body>) -> Result<Response<Body>, Infallible> {
    tracing::debug!("Health check");
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from("OK"))
        .expect("failed to build health response"))
}

pub fn routes() -> Router<Body, Infallible> {
    Router::builder().get("/", health).build().unwrap()
}
