use std::sync::Arc;

use async_graphql::{extensions, Schema};
use hyper::{Body, Response};
use routerify::Router;

use crate::{api::error::ApiErrorInterface, global::GlobalState};

pub mod error;
pub mod ext;
pub mod handlers;
pub mod models;
pub mod mutations;
pub mod queries;
pub mod subscription;

pub type MySchema = Schema<queries::Query, mutations::Mutation, subscription::Subscription>;

pub const PLAYGROUND_HTML: &str = include_str!("playground.html");

pub fn schema() -> MySchema {
    Schema::build(
        queries::Query::default(),
        mutations::Mutation::default(),
        subscription::Subscription::default(),
    )
    .enable_federation()
    .enable_subscription_in_federation()
    .extension(extensions::Analyzer)
    .limit_complexity(100) // We don't want to allow too complex queries to be executed
    .finish()
}

pub fn routes(_global: &Arc<GlobalState>) -> Router<Body, ApiErrorInterface> {
    Router::builder()
        .data(schema())
        .any_method("/", handlers::graphql_handler)
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
