use async_graphql::{ComplexObject, Context, SimpleObject};
use ulid::Ulid;

use crate::{
    api::v1::gql::{
        error::{GqlError, Result},
        ext::ContextExt,
    },
    database::{self, RolePermission, SearchResult},
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
}

/// TODO: find a better way to check if a user is allowed to read a field.

#[ComplexObject]
impl User {
    async fn email(&self, ctx: &Context<'_>) -> Result<&str> {
        let request_context = ctx.get_req_context();

        let auth = request_context.auth().await;

        if let Some(auth) = auth {
            if Ulid::from(auth.session.user_id) == *self.id
                || auth.user_permissions.has_permission(RolePermission::Admin)
            {
                return Ok(&self.email_);
            }
        }

        Err(GqlError::Unauthorized { field: "email" }.into())
    }

    async fn email_verified(&self, ctx: &Context<'_>) -> Result<bool> {
        let request_context = ctx.get_req_context();

        let auth = request_context.auth().await;

        if let Some(auth) = auth {
            if Ulid::from(auth.session.user_id) == *self.id
                || auth.user_permissions.has_permission(RolePermission::Admin)
            {
                return Ok(self.email_verified_);
            }
        }

        Err(GqlError::Unauthorized {
            field: "emailVerified",
        }
        .into())
    }

    async fn last_login_at(&self, ctx: &Context<'_>) -> Result<&DateRFC3339> {
        let request_context = ctx.get_req_context();

        let auth = request_context.auth().await;

        if let Some(auth) = auth {
            if Ulid::from(auth.session.user_id) == *self.id
                || auth.user_permissions.has_permission(RolePermission::Admin)
            {
                return Ok(&self.last_login_at_);
            }
        }

        Err(GqlError::Unauthorized {
            field: "lastLoginAt",
        }
        .into())
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
        }
    }
}
