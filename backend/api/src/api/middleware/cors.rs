use std::sync::Arc;

use hyper::http::header;
use hyper::Body;
use routerify::Middleware;

use crate::api::error::RouteError;
use crate::global::GlobalState;

pub fn cors_middleware(_: &Arc<GlobalState>) -> Middleware<Body, RouteError> {
    Middleware::post(|mut resp| async move {
        resp.headers_mut()
            .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
        resp.headers_mut().insert(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            "GET, POST, OPTIONS".parse().unwrap(),
        );
        resp.headers_mut().insert(
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            "Content-Type, Authorization".parse().unwrap(),
        );

        Ok(resp)
    })
}
