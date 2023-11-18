use std::sync::Arc;

use common::http::RouteError;
use hyper::Body;
use routerify::Router;

use super::error::ApiError;
use crate::global::ApiGlobal;

pub mod gql;

pub fn routes<G: ApiGlobal>(global: &Arc<G>) -> Router<Body, RouteError<ApiError>> {
	Router::builder()
		.scope("/gql", gql::routes(global))
		.build()
		.expect("failed to build router")
}
