use std::sync::Arc;

use async_graphql::{ComplexObject, Context, SimpleObject};

use crate::{
    api::v1::gql::{
        error::{GqlError, Result},
        GqlContext,
    },
    global::GlobalState,
};
use common::types::user;

use super::date::DateRFC3339;

#[derive(SimpleObject)]
#[graphql(complex)]
pub struct User {
    id: i64,
    username: String,
    #[graphql(skip)]
    email_: String,
    #[graphql(skip)]
    email_verified_: bool,
    created_at: DateRFC3339,
    #[graphql(skip)]
    last_login_at_: DateRFC3339,
}

/// TODO: find a better way to check if a user is allowed to read a field.

#[ComplexObject]
impl User {
    async fn email<'ctx>(&self, ctx: &Context<'_>) -> Result<&str> {
        let global = ctx
            .data::<Arc<GlobalState>>()
            .expect("failed to get global state");

        // check if the user is allowed to see the email
        let request_context = ctx
            .data::<GqlContext>()
            .expect("failed to get request context");

        let session = request_context.get_session(global).await?;

        if let Some(session) = session {
            if session.user_id == self.id {
                return Ok(&self.email_);
            }
        }

        Err(GqlError::Unauthorized
            .with_message("you are not allowed to see this field")
            .with_field(vec!["email"]))
    }

    async fn email_verified<'ctx>(&self, ctx: &Context<'_>) -> Result<bool> {
        let global = ctx
            .data::<Arc<GlobalState>>()
            .expect("failed to get global state");

        // check if the user is allowed to see the email
        let request_context = ctx
            .data::<GqlContext>()
            .expect("failed to get request context");

        let session = request_context.get_session(global).await?;

        if let Some(session) = session {
            if session.user_id == self.id {
                return Ok(self.email_verified_);
            }
        }

        Err(GqlError::Unauthorized
            .with_message("you are not allowed to see this field")
            .with_field(vec!["emailVerified"]))
    }

    async fn last_login_at<'ctx>(&self, ctx: &Context<'_>) -> Result<&DateRFC3339> {
        let global = ctx
            .data::<Arc<GlobalState>>()
            .expect("failed to get global state");

        // check if the user is allowed to see the email
        let request_context = ctx
            .data::<GqlContext>()
            .expect("failed to get request context");

        let session = request_context.get_session(global).await?;

        if let Some(session) = session {
            if session.user_id == self.id {
                return Ok(&self.last_login_at_);
            }
        }

        Err(GqlError::Unauthorized
            .with_message("you are not allowed to see this field")
            .with_field(vec!["lastLoginAt"]))
    }
}

impl From<user::Model> for User {
    fn from(value: user::Model) -> Self {
        Self {
            id: value.id,
            username: value.username,
            email_: value.email,
            email_verified_: value.email_verified,
            created_at: value.created_at.into(),
            last_login_at_: value.last_login_at.into(),
        }
    }
}
