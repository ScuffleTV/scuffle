use async_graphql::{ComplexObject, Context, SimpleObject, Union};
use bytes::Bytes;
use common::database::Ulid;
use pb::scuffle::platform::internal::two_fa::{
    two_fa_request_action::{Action, ChangePassword},
    TwoFaRequestAction,
};
use prost::Message;

use crate::api::{middleware::auth::AuthError, v1::gql::models::two_fa::TwoFaRequest};
use crate::api::v1::gql::validators::PasswordValidator;
use crate::database::TwoFaRequestActionTrait;
use crate::{
    api::v1::gql::{
        error::{GqlError, Result, ResultExt},
        ext::ContextExt,
        models::user::User,
        models::{color::Color, ulid::GqlUlid},
    },
    database,
};

mod two_fa;

#[derive(Default, SimpleObject)]
#[graphql(complex)]
pub struct UserMutation {
    two_fa: two_fa::TwoFaMutation,
}

#[derive(Clone, Union)]
pub enum ChangePasswordResponse {
    TwoFaRequest(TwoFaRequest),
    Success(User),
}

#[ComplexObject]
impl UserMutation {
    /// Change the email address of the currently logged in user.
    async fn email<'ctx>(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "New email address.", validator(email))] email: String,
    ) -> Result<User> {
        let global = ctx.get_global();
        let request_context = ctx.get_req_context();

        let auth = request_context
            .auth()
            .await?
            .map_err_gql(GqlError::Auth(AuthError::NotLoggedIn))?;

        let user: database::User = sqlx::query_as(
            "UPDATE users SET email = $1, email_verified = false, updated_at = NOW() WHERE id = $2 RETURNING *",
        )
        .bind(email)
        .bind(auth.session.user_id)
        .fetch_one(global.db.as_ref())
        .await?;

        Ok(user.into())
    }

    /// Change the display name of the currently logged in user.
    async fn display_name<'ctx>(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "New display name.")] display_name: String,
    ) -> Result<User> {
        let global = ctx.get_global();
        let request_context = ctx.get_req_context();

        let auth = request_context
            .auth()
            .await?
            .map_err_gql(GqlError::Auth(AuthError::NotLoggedIn))?;

        // TDOD: Can we combine the two queries into one?
        let user: database::User = global
            .user_by_id_loader
            .load(auth.session.user_id.0)
            .await
            .map_err_gql("failed to fetch user")?
            .map_err_gql(GqlError::NotFound("user"))?;

        // Check case
        if user.username.to_lowercase() != display_name.to_lowercase() {
            return Err(GqlError::InvalidInput {
                fields: vec!["displayName"],
                message: "Display name must match username case",
            }
            .into());
        }

        let user: database::User = sqlx::query_as(
            "UPDATE users SET display_name = $1, updated_at = NOW() WHERE id = $2 AND username = $3 RETURNING *",
        )
        .bind(display_name.clone())
        .bind(auth.session.user_id)
        .bind(user.username)
        .fetch_one(global.db.as_ref())
        .await?;

        let user_id = user.id.0.to_string();

        global
            .nats
            .publish(
                format!("user.{}.display_name", user_id),
                pb::scuffle::platform::internal::events::UserDisplayName {
                    user_id,
                    display_name,
                }
                .encode_to_vec()
                .into(),
            )
            .await
            .map_err(|_| "Failed to publish message")?;

        Ok(user.into())
    }

    /// Change the display color of the currently logged in user.
    async fn display_color<'ctx>(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "New display color.")] color: Color,
    ) -> Result<User> {
        let global = ctx.get_global();
        let request_context = ctx.get_req_context();

        let auth = request_context
            .auth()
            .await?
            .ok_or(GqlError::Auth(AuthError::NotLoggedIn))?;

        let user: database::User = sqlx::query_as(
            "UPDATE users SET display_color = $1, updated_at = NOW() WHERE id = $2 RETURNING *",
        )
        .bind(*color)
        .bind(auth.session.user_id)
        .fetch_one(global.db.as_ref())
        .await?;

        let user_id = user.id.0.to_string();

        global
            .nats
            .publish(
                format!("user.{}.display_color", user_id),
                pb::scuffle::platform::internal::events::UserDisplayColor {
                    user_id,
                    display_color: *color,
                }
                .encode_to_vec()
                .into(),
            )
            .await
            .map_err_gql("failed to publish message")?;

        Ok(user.into())
    }

    async fn password<'ctx>(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Current password")] current_password: String,
        #[graphql(desc = "New password", validator(custom = "PasswordValidator"))]
        new_password: String,
    ) -> Result<ChangePasswordResponse> {
        let global = ctx.get_global();
        let request_context = ctx.get_req_context();

        let auth = request_context
            .auth()
            .await?
            .ok_or(GqlError::Auth(AuthError::NotLoggedIn))?;

        let user = global
            .user_by_id_loader
            .load(auth.session.user_id.0)
            .await
            .map_err_gql("failed to fetch user")?
            .map_err_gql(GqlError::NotFound("user"))?;

        if !user.verify_password(&current_password) {
            return Err(GqlError::InvalidInput {
                fields: vec!["password"],
                message: "wrong password",
            }
            .into());
        }

        let change_password = ChangePassword {
            new_password_hash: database::User::hash_password(&new_password),
            current_session_id: Some(auth.session.id.0.into()),
        };

        if user.totp_enabled {
            let request_id = ulid::Ulid::new();
            sqlx::query("INSERT INTO two_fa_requests (id, user_id, action) VALUES ($1, $2, $3)")
                .bind(Ulid::from(request_id))
                .bind(user.id)
                .bind(TwoFaRequestAction { action: Some(Action::ChangePassword(change_password)) }.encode_to_vec())
                .execute(global.db.as_ref())
                .await?;
            Ok(ChangePasswordResponse::TwoFaRequest(TwoFaRequest { id: request_id.into() }))
        } else {
            change_password.execute(global, user.id).await?;
            Ok(ChangePasswordResponse::Success(user.into()))
        }
    }

    /// Follow or unfollow a user.
    async fn follow<'ctx>(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The channel to (un)follow.")] channel_id: GqlUlid,
        #[graphql(desc = "Set to true for follow and false for unfollow")] follow: bool,
    ) -> Result<bool> {
        let global = ctx.get_global();
        let request_context = ctx.get_req_context();

        let auth = request_context
            .auth()
            .await?
            .ok_or(GqlError::Auth(AuthError::NotLoggedIn))?;

        if auth.session.user_id.0 == channel_id.to_ulid() {
            return Err(GqlError::InvalidInput {
                fields: vec!["channelId"],
                message: "Cannot follow yourself",
            }
            .into());
        }

        sqlx::query("UPSERT INTO channel_user(user_id, channel_id, following) VALUES ($1, $2, $3)")
            .bind(auth.session.user_id)
            .bind(channel_id.to_uuid())
            .bind(follow)
            .execute(global.db.as_ref())
            .await?;

        let user_id = auth.session.user_id.0.to_string();
        let channel_id = channel_id.to_string();

        let user_subject = format!("user.{}.follows", user_id);
        let channel_subject = format!("channel.{}.follows", channel_id);

        let msg = Bytes::from(
            pb::scuffle::platform::internal::events::UserFollowChannel {
                user_id,
                channel_id,
                following: follow,
            }
            .encode_to_vec(),
        );

        global
            .nats
            .publish(user_subject, msg.clone())
            .await
            .map_err_gql("failed to publish message")?;

        global
            .nats
            .publish(channel_subject, msg)
            .await
            .map_err_gql("failed to publish message")?;

        Ok(follow)
    }
}
