use async_graphql::{Context, Object};
use bytes::Bytes;
use prost::Message;

use crate::{
    api::v1::gql::{
        error::{GqlError, Result, ResultExt},
        ext::ContextExt,
        models::user::User,
        models::{color::Color, ulid::GqlUlid},
    },
    database,
};

#[derive(Default)]
pub struct UserMutation;

#[Object]
impl UserMutation {
    /// Change the email address of the currently logged in user.
    async fn email<'ctx>(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "New email address.")] email: String,
    ) -> Result<User> {
        let global = ctx.get_global();
        let request_context = ctx.get_req_context();

        let auth = request_context
            .auth()
            .await
            .map_err_gql(GqlError::NotLoggedIn)?;

        database::User::validate_email(&email).map_err(|e| GqlError::InvalidInput {
            fields: vec!["email"],
            message: e,
        })?;

        let user: database::User = sqlx::query_as(
            "UPDATE users SET email = $1, email_verified = false WHERE id = $2 RETURNING *",
        )
        .bind(email)
        .bind(auth.session.user_id)
        .fetch_one(global.db.as_ref())
        .await
        .map_err_gql("failed to update user")?;

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
            .await
            .map_err_gql(GqlError::NotLoggedIn)?;

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
                fields: vec!["display_name"],
                message: "Display name must match username case",
            }
            .into());
        }

        let user: database::User = sqlx::query_as(
            "UPDATE users SET display_name = $1 WHERE id = $2 AND username = $3 RETURNING *",
        )
        .bind(display_name.clone())
        .bind(auth.session.user_id)
        .bind(user.username)
        .fetch_one(global.db.as_ref())
        .await
        .map_err_gql("failed to update user")?;

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

        let auth = request_context.auth().await.ok_or(GqlError::NotLoggedIn)?;

        let user: database::User =
            sqlx::query_as("UPDATE users SET display_color = $1 WHERE id = $2 RETURNING *")
                .bind(*color)
                .bind(auth.session.user_id)
                .fetch_one(global.db.as_ref())
                .await
                .map_err_gql("failed to update user")?;

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

    /// Follow or unfollow a user.
    async fn follow<'ctx>(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The channel to (un)follow.")] channel_id: GqlUlid,
        #[graphql(desc = "Set to true for follow and false for unfollow")] follow: bool,
    ) -> Result<bool> {
        let global = ctx.get_global();
        let request_context = ctx.get_req_context();

        let auth = request_context.auth().await.ok_or(GqlError::NotLoggedIn)?;

        if auth.session.user_id.0 == channel_id.to_ulid() {
            return Err(GqlError::InvalidInput {
                fields: vec!["channel_id"],
                message: "Cannot follow yourself",
            }
            .into());
        }

        sqlx::query("UPSERT INTO channel_user(user_id, channel_id, following) VALUES ($1, $2, $3)")
            .bind(auth.session.user_id)
            .bind(channel_id.to_uuid())
            .bind(follow)
            .execute(global.db.as_ref())
            .await
            .map_err_gql("failed to update follow")?;

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
