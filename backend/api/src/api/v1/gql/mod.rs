use std::sync::Arc;

use arc_swap::ArcSwap;
use async_graphql::{
    extensions, futures_util::Stream, ComplexObject, Context, Schema, SimpleObject, Subscription,
};
use common::types::session;
use hyper::{Body, Response};
use routerify::Router;

use crate::{api::error::RouteError, global::GlobalState};

use self::error::{GqlError, Result, ResultExt};

pub mod auth;
pub mod error;
pub mod handlers;
pub mod models;

#[derive(Default)]
pub struct GqlContext {
    pub is_websocket: bool,
    pub session: ArcSwap<Option<session::Model>>,
}

impl GqlContext {
    pub async fn get_session(&self, global: &Arc<GlobalState>) -> Result<Option<session::Model>> {
        let guard = self.session.load();
        let Some(session) = guard.as_ref() else {
            return Ok(None)
        };

        if !self.is_websocket {
            if !session.is_valid() {
                return Err(GqlError::InvalidSession.with_message("Session is no longer valid"));
            }

            return Ok(Some(session.clone()));
        }

        let session = global
            .session_by_id_loader
            .load_one(session.id)
            .await
            .map_err_gql("failed to fetch session")?
            .and_then(|s| if s.is_valid() { Some(s) } else { None })
            .ok_or_else(|| {
                self.session.store(Arc::new(None));
                GqlError::InvalidSession.with_message("Session is no longer valid")
            })?;

        self.session.store(Arc::new(Some(session.clone())));

        Ok(Some(session))
    }
}

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
        let global = ctx
            .data::<Arc<GlobalState>>()
            .expect("failed to get global state");

        let user = global
            .user_by_username_loader
            .load_one(username.to_lowercase())
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
    .extension(extensions::ApolloTracing)
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
