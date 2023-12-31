use std::sync::Arc;

use common::http::router::builder::RouterBuilder;
use common::http::router::Router;
use common::http::RouteError;
use hyper::body::Incoming;

use super::error::ApiError;
use super::Body;
use crate::global::ApiGlobal;

pub mod gql;
pub mod upload;

pub fn routes<G: ApiGlobal>(global: &Arc<G>) -> RouterBuilder<Incoming, Body, RouteError<ApiError>> {
	Router::builder()
		.scope("/gql", gql::routes(global))
		.scope("/upload", upload::routes(global))
}
