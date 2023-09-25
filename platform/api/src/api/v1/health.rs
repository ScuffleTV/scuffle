use std::sync::Arc;

use hyper::{Body, Request, Response, StatusCode};
use routerify::Router;
use serde_json::json;

use crate::{
    api::{
        error::{ApiErrorInterface, Result},
        macros::make_response,
    },
    global::GlobalState,
};

async fn health(_: Request<Body>) -> Result<Response<Body>> {
    Ok(make_response!(
        StatusCode::OK,
        json!({
            "status": "ok"
        })
    ))
}

pub fn routes(_global: &Arc<GlobalState>) -> Router<Body, ApiErrorInterface> {
    Router::builder()
        .get("/", health)
        .build()
        .expect("failed to build router")
}
