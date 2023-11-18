use std::sync::Arc;

use common::http::RouteError;
use hyper::http::header;
use hyper::Body;
use routerify::Middleware;

use crate::api::error::ApiError;
use crate::global::ApiGlobal;

pub fn cors_middleware<G: ApiGlobal>(_: &Arc<G>) -> Middleware<Body, RouteError<ApiError>> {
	Middleware::post(|mut resp| async move {
		resp.headers_mut()
			.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
		resp.headers_mut()
			.insert(header::ACCESS_CONTROL_ALLOW_METHODS, "GET, POST, OPTIONS".parse().unwrap());
		resp.headers_mut().insert(
			header::ACCESS_CONTROL_ALLOW_HEADERS,
			"Content-Type, Authorization".parse().unwrap(),
		);

		Ok(resp)
	})
}
