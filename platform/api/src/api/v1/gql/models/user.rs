use async_graphql::{ComplexObject, Context, SimpleObject};

use crate::{
    api::v1::gql::{error::Result, guards::auth_guard},
    database::{self, SearchResult},
};

use super::{channel::Channel, color::DisplayColor, date::DateRFC3339, ulid::GqlUlid};

#[derive(SimpleObject, Clone)]
pub struct UserSearchResult {
    user: User,
    similarity: f64,
}

impl From<SearchResult<database::User>> for UserSearchResult {
    fn from(value: SearchResult<database::User>) -> Self {
        Self {
            user: value.object.into(),
            similarity: value.similarity,
        }
    }
}

#[derive(SimpleObject, Clone)]
#[graphql(complex)]
pub struct User {
    pub id: GqlUlid,
    pub display_name: String,
    pub display_color: DisplayColor,
    pub username: String,
    pub channel: Channel,

    // Private fields
    #[graphql(skip)]
    pub email_: String,
    #[graphql(skip)]
    pub email_verified_: bool,
    #[graphql(skip)]
    pub last_login_at_: DateRFC3339,
    #[graphql(skip)]
    pub totp_enabled_: bool,
}

#[ComplexObject]
impl User {
    async fn email(&self, ctx: &Context<'_>) -> Result<&String> {
        auth_guard(ctx, "email", &self.email_, self.id.into()).await
    }

    async fn email_verified(&self, ctx: &Context<'_>) -> Result<bool> {
        auth_guard(ctx, "emailVerified", self.email_verified_, self.id.into()).await
    }

    async fn last_login_at(&self, ctx: &Context<'_>) -> Result<&DateRFC3339> {
        auth_guard(ctx, "lastLoginAt", &self.last_login_at_, self.id.into()).await
    }

    async fn totp_enabled(&self, ctx: &Context<'_>) -> Result<bool> {
        auth_guard(ctx, "totpEnabled", self.totp_enabled_, self.id.into()).await
    }
}

impl From<database::User> for User {
    fn from(value: database::User) -> Self {
        Self {
            id: value.id.0.into(),
            username: value.username,
            display_name: value.display_name,
            display_color: value.display_color.into(),
            channel: value.channel.into(),
            email_: value.email,
            email_verified_: value.email_verified,
            last_login_at_: value.last_login_at.into(),
            totp_enabled_: value.totp_enabled,
        }
    }
}
