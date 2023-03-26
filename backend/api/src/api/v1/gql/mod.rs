use std::sync::Arc;

use async_graphql::{
    extensions, futures_util::Stream, ComplexObject, Context, Schema, SimpleObject, Subscription,
};
use hyper::{Body, Response};
use routerify::Router;
use uuid::Uuid;

use crate::{api::error::RouteError, global::GlobalState};

use self::{
    error::{Result, ResultExt},
    ext::ContextExt,
};

pub mod auth;
pub mod error;
pub mod ext;
pub mod handlers;
pub mod models;
pub mod request_context;

#[derive(Default, SimpleObject)]
#[graphql(complex)]
/// The root query type which contains root level fields.
pub struct Query {
    noop: bool,
}

#[derive(Default, SimpleObject)]
/// The root mutation type which contains root level fields.
pub struct Mutation {
    auth: auth::AuthMutation,
}

#[derive(Default)]
/// The root subscription type which contains root level fields.
pub struct Subscription {}

#[Subscription]
impl Subscription {
    async fn noop(&self) -> Result<impl Stream<Item = bool>> {
        Ok(futures_util::stream::iter(Vec::new()))
    }
}

#[ComplexObject]
impl Query {
    async fn user_by_username(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The username of the user.")] username: String,
    ) -> Result<Option<models::user::User>> {
        let global = ctx.get_global();

        let user = global
            .user_by_username_loader
            .load_one(username.to_lowercase())
            .await
            .map_err_gql("failed to fetch user")?;

        Ok(user.map(models::user::User::from))
    }

    async fn user_by_id(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The id of the user.")] id: Uuid,
    ) -> Result<Option<models::user::User>> {
        let global = ctx.get_global();

        let user = global
            .user_by_id_loader
            .load_one(id)
            .await
            .map_err_gql("failed to fetch user")?;

        Ok(user.map(models::user::User::from))
    }
}

pub type MySchema = Schema<Query, Mutation, Subscription>;

pub const PLAYGROUND_HTML: &str = include_str!("playground.html");

pub fn schema() -> MySchema {
    Schema::build(
        Query::default(),
        Mutation::default(),
        Subscription::default(),
    )
    .enable_federation()
    .enable_subscription_in_federation()
    .extension(extensions::Analyzer)
    .limit_complexity(100) // We don't want to allow too complex queries to be executed
    .finish()
}

pub fn routes(_global: &Arc<GlobalState>) -> Router<Body, RouteError> {
    let router = Router::builder()
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
        .expect("failed to build router");

    router
}
