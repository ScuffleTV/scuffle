use std::sync::Arc;

use hyper::body::Incoming;
use hyper::http::header;
use serde_json::json;
use utils::http::router::extend::{extend_fn, ExtendRouter};
use utils::http::router::middleware::Middleware;
use utils::http::RouteError;
use utils::make_response;

use crate::api::error::ApiError;
use crate::api::Body;
use crate::global::ApiGlobal;

pub fn cors_middleware<G: ApiGlobal>(_: &Arc<G>) -> impl ExtendRouter<Incoming, Body, RouteError<ApiError>> {
	extend_fn(|router| {
		router
			.middleware(Middleware::post(|mut resp| async move {
				resp.headers_mut()
					.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
				resp.headers_mut()
					.insert(header::ACCESS_CONTROL_ALLOW_METHODS, "GET, POST, OPTIONS".parse().unwrap());
				resp.headers_mut().insert(
					header::ACCESS_CONTROL_ALLOW_HEADERS,
					"Content-Type, Authorization".parse().unwrap(),
				);

				Ok(resp)
			}))
			.options("/*", |_| async move {
				Ok(make_response!(
					hyper::StatusCode::OK,
					json!({
						"success": true,
					})
				))
			})
	})
}
