use std::sync::Arc;

use async_graphql::{extensions, Schema};
use common::http::RouteError;
use hyper::{Body, Response};
use routerify::Router;

use crate::{api::error::ApiError, global::ApiGlobal};

mod error;
mod ext;
mod guards;
mod handlers;
mod models;
mod mutations;
mod queries;
mod subscription;
mod validators;

pub type MySchema<G> =
    Schema<queries::Query<G>, mutations::Mutation<G>, subscription::Subscription<G>>;

pub const PLAYGROUND_HTML: &str = include_str!("playground.html");

pub fn schema<G: ApiGlobal>() -> MySchema<G> {
    Schema::build(
        queries::Query::<G>::default(),
        mutations::Mutation::<G>::default(),
        subscription::Subscription::<G>::default(),
    )
    .enable_federation()
    .enable_subscription_in_federation()
    .extension(extensions::Analyzer)
    .extension(extensions::Tracing)
    .limit_complexity(100) // We don't want to allow too complex queries to be executed
    .finish()
}

pub fn routes<G: ApiGlobal>(_: &Arc<G>) -> Router<Body, RouteError<ApiError>> {
    Router::builder()
        .data(schema::<G>())
        .any_method("/", handlers::graphql_handler::<G>)
        .get("/playground", move |_| async move {
            Ok(Response::builder()
                .status(200)
                .header("content-type", "text/html")
                .body(Body::from(PLAYGROUND_HTML))
                .expect("failed to build response"))
        })
        .build()
        .expect("failed to build router")
}