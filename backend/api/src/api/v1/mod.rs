use std::sync::Arc;

use hyper::Body;
use routerify::Router;

use crate::global::GlobalState;

use super::error::RouteError;

pub mod gql;
pub mod health;
pub mod jwt;
pub mod middleware;

pub fn routes(global: &Arc<GlobalState>) -> Router<Body, RouteError> {
    Router::builder()
        .scope("/health", health::routes(global))
        .middleware(middleware::auth::auth_middleware(global))
        .scope("/gql", gql::routes(global))
        .build()
        .expect("failed to build router")
}
