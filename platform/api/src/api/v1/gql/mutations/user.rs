use async_graphql::{Context, Object};
use bytes::Bytes;
use common::database::Ulid;
use pb::scuffle::platform::internal::two_fa::two_fa_request_action::{Action, ChangePassword};
use pb::scuffle::platform::internal::two_fa::TwoFaRequestAction;
use prost::Message;

use crate::api::auth::AuthError;
use crate::api::v1::gql::error::ext::*;
use crate::api::v1::gql::error::{GqlError, Result};
use crate::api::v1::gql::ext::ContextExt;
use crate::api::v1::gql::models::color::Color;
use crate::api::v1::gql::models::two_fa::{TwoFaRequest, TwoFaResponse};
use crate::api::v1::gql::models::ulid::GqlUlid;
use crate::api::v1::gql::models::user::User;
use crate::api::v1::gql::validators::PasswordValidator;
use crate::database;
use crate::database::TwoFaRequestActionTrait;
use crate::global::ApiGlobal;
use crate::subscription::SubscriptionTopic;

mod two_fa;

pub struct UserMutation<G: ApiGlobal> {
	two_fa: two_fa::TwoFaMutation<G>,
}

impl<G: ApiGlobal> Default for UserMutation<G> {
	fn default() -> Self {
		Self {
			two_fa: Default::default(),
		}
	}
}

#[Object]
impl<G: ApiGlobal> UserMutation<G> {
	/// Change the email address of the currently logged in user.
	async fn email<'ctx>(
		&self,
		ctx: &Context<'_>,
		#[graphql(desc = "New email address.", validator(email))] email: String,
	) -> Result<User<G>> {
		let global = ctx.get_global::<G>();
		let request_context = ctx.get_req_context();

		let auth = request_context
			.auth(global)
			.await?
			.map_err_gql(GqlError::Auth(AuthError::NotLoggedIn))?;

		let user: database::User = sqlx::query_as(
			r#"
			UPDATE users
			SET
				email = $1,
				email_verified = false,
				updated_at = NOW()
			WHERE
				id = $2
			RETURNING *
			"#,
		)
		.bind(email)
		.bind(auth.session.user_id)
		.fetch_one(global.db().as_ref())
		.await?;

		Ok(user.into())
	}

	/// Change the display name of the currently logged in user.
	async fn display_name<'ctx>(
		&self,
		ctx: &Context<'_>,
		#[graphql(desc = "New display name.")] display_name: String,
	) -> Result<User<G>> {
		let global = ctx.get_global::<G>();
		let request_context = ctx.get_req_context();

		let auth = request_context
			.auth(global)
			.await?
			.map_err_gql(GqlError::Auth(AuthError::NotLoggedIn))?;

		// TDOD: Can we combine the two queries into one?
		let user: database::User = global
			.user_by_id_loader()
			.load(auth.session.user_id.0)
			.await
			.map_err_ignored_gql("failed to fetch user")?
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
			r#"
			UPDATE users
			SET
				display_name = $1,
				updated_at = NOW()
			WHERE
				id = $2
				AND username = $3
			RETURNING *
			"#,
		)
		.bind(display_name.clone())
		.bind(auth.session.user_id)
		.bind(user.username)
		.fetch_one(global.db().as_ref())
		.await?;

		global
			.nats()
			.publish(
				SubscriptionTopic::UserDisplayName(user.id.0),
				pb::scuffle::platform::internal::events::UserDisplayName {
					user_id: Some(user.id.0.into()),
					display_name,
				}
				.encode_to_vec()
				.into(),
			)
			.await
			.map_err(|_| "failed to publish message")?;

		Ok(user.into())
	}

	/// Change the display color of the currently logged in user.
	async fn display_color<'ctx>(
		&self,
		ctx: &Context<'_>,
		#[graphql(desc = "New display color.")] color: Color,
	) -> Result<User<G>> {
		let global = ctx.get_global::<G>();
		let request_context = ctx.get_req_context();

		let auth = request_context
			.auth(global)
			.await?
			.ok_or(GqlError::Auth(AuthError::NotLoggedIn))?;

		let user: database::User = sqlx::query_as(
			r#"
				UPDATE users
				SET
					display_color = $1,
					updated_at = NOW()
				WHERE
					id = $2
				RETURNING *
				"#,
		)
		.bind(*color)
		.bind(auth.session.user_id)
		.fetch_one(global.db().as_ref())
		.await?;

		global
			.nats()
			.publish(
				SubscriptionTopic::UserDisplayColor(user.id.0),
				pb::scuffle::platform::internal::events::UserDisplayColor {
					user_id: Some(user.id.0.into()),
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
		#[graphql(desc = "New password", validator(custom = "PasswordValidator"))] new_password: String,
	) -> Result<TwoFaResponse<User<G>>> {
		let global = ctx.get_global::<G>();
		let request_context = ctx.get_req_context();

		let auth = request_context
			.auth(global)
			.await?
			.ok_or(GqlError::Auth(AuthError::NotLoggedIn))?;

		let user = global
			.user_by_id_loader()
			.load(auth.session.user_id.0)
			.await
			.map_err_ignored_gql("failed to fetch user")?
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
			sqlx::query(
				r#"
				INSERT INTO two_fa_requests (
					id,
					user_id,
					action
				) VALUES (
					$1,
					$2,
					$3
				)
				"#,
			)
			.bind(Ulid::from(request_id))
			.bind(user.id)
			.bind(
				TwoFaRequestAction {
					action: Some(Action::ChangePassword(change_password)),
				}
				.encode_to_vec(),
			)
			.execute(global.db().as_ref())
			.await?;
			Ok(TwoFaResponse::TwoFaRequest(TwoFaRequest { id: request_id.into() }))
		} else {
			change_password.execute(global, user.id).await?;
			Ok(TwoFaResponse::Success(user.into()))
		}
	}

	/// Follow or unfollow a user.
	async fn follow<'ctx>(
		&self,
		ctx: &Context<'_>,
		#[graphql(desc = "The channel to (un)follow.")] channel_id: GqlUlid,
		#[graphql(desc = "Set to true for follow and false for unfollow")] follow: bool,
	) -> Result<bool> {
		let global = ctx.get_global::<G>();
		let request_context = ctx.get_req_context();

		let auth = request_context
			.auth(global)
			.await?
			.ok_or(GqlError::Auth(AuthError::NotLoggedIn))?;

		if auth.session.user_id.0 == channel_id.to_ulid() {
			return Err(GqlError::InvalidInput {
				fields: vec!["channelId"],
				message: "Cannot follow yourself",
			}
			.into());
		}

		sqlx::query(
			r#"
			UPSERT INTO channel_user (
				user_id,
				channel_id,
				following
			) VALUES (
				$1,
				$2,
				$3
			)
			"#,
		)
		.bind(auth.session.user_id)
		.bind(channel_id.to_uuid())
		.bind(follow)
		.execute(global.db().as_ref())
		.await?;

		let channel_id = channel_id.to_ulid();
		let user_subject = SubscriptionTopic::UserFollows(auth.session.user_id.0);
		let channel_subject = SubscriptionTopic::ChannelFollows(channel_id);

		let msg = Bytes::from(
			pb::scuffle::platform::internal::events::UserFollowChannel {
				user_id: Some(auth.session.user_id.0.into()),
				channel_id: Some(channel_id.into()),
				following: follow,
			}
			.encode_to_vec(),
		);

		global
			.nats()
			.publish(user_subject, msg.clone())
			.await
			.map_err_gql("failed to publish message")?;

		global
			.nats()
			.publish(channel_subject, msg)
			.await
			.map_err_gql("failed to publish message")?;

		Ok(follow)
	}

	#[inline(always)]
	async fn two_fa(&self) -> &two_fa::TwoFaMutation<G> {
		&self.two_fa
	}
}
