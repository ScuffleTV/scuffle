use std::sync::Arc;

use hyper::body::Incoming;
use scuffle_utils::http::router::builder::RouterBuilder;
use scuffle_utils::http::router::Router;
use scuffle_utils::http::RouteError;

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
