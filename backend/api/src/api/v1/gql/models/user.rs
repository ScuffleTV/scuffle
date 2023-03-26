use async_graphql::{ComplexObject, Context, SimpleObject};
use uuid::Uuid;

use crate::api::v1::gql::{
    error::{GqlError, Result},
    ext::ContextExt,
};
use crate::database::{global_role, user};

use super::{date::DateRFC3339, global_roles::GlobalRole};

#[derive(SimpleObject, Clone)]
#[graphql(complex)]
pub struct User {
    pub id: Uuid,
    pub display_name: String,
    pub username: String,
    pub created_at: DateRFC3339,

    // Private fields
    #[graphql(skip)]
    pub email_: String,
    #[graphql(skip)]
    pub email_verified_: bool,
    #[graphql(skip)]
    pub last_login_at_: DateRFC3339,
    #[graphql(skip)]
    pub stream_key_: String,
}

/// TODO: find a better way to check if a user is allowed to read a field.

#[ComplexObject]
impl User {
    async fn email<'ctx>(&self, ctx: &Context<'_>) -> Result<&str> {
        let global = ctx.get_global();
        let request_context = ctx.get_session();

        let session = request_context.get_session(global).await?;

        if let Some((session, perms)) = session {
            if session.user_id == self.id
                || perms
                    .permissions
                    .has_permission(global_role::Permission::Admin)
            {
                return Ok(&self.email_);
            }
        }

        Err(GqlError::Unauthorized
            .with_message("you are not allowed to see this field")
            .with_field(vec!["email"]))
    }

    async fn email_verified<'ctx>(&self, ctx: &Context<'_>) -> Result<bool> {
        let global = ctx.get_global();
        let request_context = ctx.get_session();

        let session = request_context.get_session(global).await?;

        if let Some((session, perms)) = session {
            if session.user_id == self.id
                || perms
                    .permissions
                    .has_permission(global_role::Permission::Admin)
            {
                return Ok(self.email_verified_);
            }
        }

        Err(GqlError::Unauthorized
            .with_message("you are not allowed to see this field")
            .with_field(vec!["emailVerified"]))
    }

    async fn last_login_at<'ctx>(&self, ctx: &Context<'_>) -> Result<&DateRFC3339> {
        let global = ctx.get_global();
        let request_context = ctx.get_session();

        let session = request_context.get_session(global).await?;

        if let Some((session, perms)) = session {
            if session.user_id == self.id
                || perms
                    .permissions
                    .has_permission(global_role::Permission::Admin)
            {
                return Ok(&self.last_login_at_);
            }
        }

        Err(GqlError::Unauthorized
            .with_message("you are not allowed to see this field")
            .with_field(vec!["lastLoginAt"]))
    }

    async fn stream_key<'ctx>(&self, ctx: &Context<'_>) -> Result<&str> {
        let global = ctx.get_global();
        let request_context = ctx.get_session();

        let session = request_context.get_session(global).await?;

        if let Some((session, perms)) = session {
            if session.user_id == self.id
                || perms
                    .permissions
                    .has_permission(global_role::Permission::Admin)
            {
                return Ok(&self.stream_key_);
            }
        }

        Err(GqlError::Unauthorized
            .with_message("you are not allowed to see this field")
            .with_field(vec!["stream_key"]))
    }

    async fn permissions<'ctx>(&self, ctx: &Context<'_>) -> Result<i64> {
        let global = ctx.get_global();

        let global_roles = global
            .user_permisions_by_id_loader
            .load_one(self.id)
            .await
            .map_err(|e| {
                tracing::error!("failed to fetch global roles: {}", e);

                GqlError::InternalServerError
                    .with_message("failed to fetch global roles")
                    .with_field(vec!["permissions"])
            })?
            .map(|p| p.permissions)
            .unwrap_or_default();

        Ok(global_roles.bits())
    }

    async fn global_roles<'ctx>(&self, ctx: &Context<'_>) -> Result<Vec<GlobalRole>> {
        let global = ctx.get_global();

        let global_roles = global
            .user_permisions_by_id_loader
            .load_one(self.id)
            .await
            .map_err(|e| {
                tracing::error!("failed to fetch global roles: {}", e);

                GqlError::InternalServerError
                    .with_message("failed to fetch global roles")
                    .with_field(vec!["globalRoles"])
            })?
            .map(|p| p.roles.into_iter().map(GlobalRole::from).collect())
            .unwrap_or_default();

        Ok(global_roles)
    }
}

impl From<user::Model> for User {
    fn from(value: user::Model) -> Self {
        let stream_key = value.get_stream_key();
        Self {
            id: value.id,
            username: value.username,
            display_name: value.display_name,
            email_: value.email,
            email_verified_: value.email_verified,
            created_at: value.created_at.into(),
            last_login_at_: value.last_login_at.into(),
            stream_key_: stream_key,
        }
    }
}
