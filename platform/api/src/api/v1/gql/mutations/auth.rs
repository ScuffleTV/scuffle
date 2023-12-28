use std::collections::HashMap;

use async_graphql::{Context, Object, Union};
use chrono::{Duration, Utc};
use common::database::{TraitProtobuf, Ulid};
use pb::scuffle::platform::internal::two_fa::two_fa_request_action::{Action, Login};
use pb::scuffle::platform::internal::two_fa::TwoFaRequestAction;
use prost::Message;

use crate::api::auth::{AuthData, AuthError};
use crate::api::jwt::{AuthJwtPayload, JwtState};
use crate::api::v1::gql::error::ext::*;
use crate::api::v1::gql::error::{GqlError, Result};
use crate::api::v1::gql::ext::ContextExt;
use crate::api::v1::gql::models::session::Session;
use crate::api::v1::gql::models::two_fa::{TwoFaRequest, TwoFaResponse};
use crate::api::v1::gql::models::ulid::GqlUlid;
use crate::api::v1::gql::validators::{PasswordValidator, UsernameValidator};
use crate::database;
use crate::database::TwoFaRequestActionTrait;
use crate::global::ApiGlobal;
use crate::turnstile::validate_turnstile_token;

#[derive(Clone)]
pub struct AuthMutation<G>(std::marker::PhantomData<G>);

impl<G: ApiGlobal> Default for AuthMutation<G> {
	fn default() -> Self {
		Self(std::marker::PhantomData)
	}
}

#[derive(Union)]
pub enum TwoFaRequestFulfillResponse {
	Login(Session),
}

#[Object]
/// The mutation object for authentication
impl<G: ApiGlobal> AuthMutation<G> {
	/// Login using a username and password. If via websocket this will
	/// authenticate the websocket connection.
	async fn login<'ctx>(
		&self,
		ctx: &Context<'_>,
		#[graphql(desc = "The username of the user.")] username: String,
		#[graphql(desc = "The password of the user.")] password: String,
		#[graphql(desc = "The captcha token from cloudflare turnstile.")] captcha_token: String,
		#[graphql(desc = "The duration of the session in seconds. If not specified it will be 7 days.")] validity: Option<
			u32,
		>,
		#[graphql(desc = "Setting this to false will make it so logging in does not authenticate the connection.")]
		update_context: Option<bool>,
	) -> Result<TwoFaResponse<Session>> {
		let global = ctx.get_global::<G>();
		let request_context = ctx.get_req_context();

		if !validate_turnstile_token(global, &captcha_token)
			.await
			.map_err_gql("failed to validate captcha token")?
		{
			return Err(GqlError::InvalidInput {
				fields: vec!["captchaToken"],
				message: "capcha token is invalid",
			}
			.into());
		}

		let user = global
			.user_by_username_loader()
			.load(username.to_lowercase())
			.await
			.map_err_ignored_gql("failed to fetch user")?
			.map_err_gql(GqlError::InvalidInput {
				fields: vec!["username", "password"],
				message: "invalid username or password",
			})?;

		if !user.verify_password(&password) {
			return Err(GqlError::InvalidInput {
				fields: vec!["username", "password"],
				message: "invalid username or password",
			}
			.into());
		}

		let login = Login {
			login_duration: validity.unwrap_or(60 * 60 * 24 * 7), // 7 days
			update_context: update_context.unwrap_or(true),
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
				)"#,
			)
			.bind(Ulid::from(request_id))
			.bind(user.id)
			.bind(
				TwoFaRequestAction {
					action: Some(Action::Login(login)),
				}
				.encode_to_vec(),
			)
			.execute(global.db().as_ref())
			.await?;
			Ok(TwoFaResponse::TwoFaRequest(TwoFaRequest { id: request_id.into() }))
		} else {
			let session = login.execute(global, user.id).await?;

			let jwt = AuthJwtPayload::from(session.clone());
			let token = jwt
				.serialize(global)
				.ok_or(GqlError::InternalServerError("failed to serialize JWT"))?;

			// We need to update the request context with the new session
			if update_context.unwrap_or(true) {
				let auth_data = AuthData::from_session_and_user(global, session.clone(), &user).await?;
				request_context.set_auth(auth_data).await;
			}

			Ok(TwoFaResponse::Success(Session {
				id: session.id.0.into(),
				token,
				user_id: session.user_id.0.into(),
				expires_at: session.expires_at.into(),
				last_used_at: session.last_used_at.into(),
			}))
		}
	}

	/// Fulfill a two-factor authentication request.
	async fn fulfill_two_fa_request<'ctx>(
		&self,
		ctx: &Context<'_>,
		#[graphql(desc = "ID of the 2fa request to be fulfilled.")] id: GqlUlid,
		#[graphql(desc = "The TOTP code.")] code: String,
	) -> Result<Option<TwoFaRequestFulfillResponse>> {
		let global = ctx.get_global::<G>();
		let request_context = ctx.get_req_context();

		// TODO: Make this a dataloader
		let request: database::TwoFaRequest = sqlx::query_as(
			r#"
			SELECT
				*
			FROM
				two_fa_requests
			WHERE
				id = $1
			"#,
		)
		.bind(Ulid::from(id.to_ulid()))
		.fetch_optional(global.db().as_ref())
		.await?
		.ok_or(GqlError::NotFound("2fa request"))?;

		let user = global
			.user_by_id_loader()
			.load(request.user_id.into())
			.await
			.map_err_ignored_gql("failed to fetch user")?
			.ok_or(GqlError::NotFound("user"))?;

		if !user.verify_totp_code(&code, true)? {
			return Err(GqlError::InvalidInput {
				fields: vec!["code"],
				message: "wrong code",
			}
			.into());
		}

		sqlx::query(
			r#"
			DELETE FROM
				two_fa_requests
			WHERE
				id = $1
			"#,
		)
		.bind(request.id)
		.execute(global.db().as_ref())
		.await?;

		match request.action.into_inner().action {
			Some(Action::Login(action)) => {
				let update_context = action.update_context;
				let session = action.execute(global, user.id).await?;
				let jwt = AuthJwtPayload::from(session.clone());
				let token = jwt
					.serialize(global)
					.ok_or(GqlError::InternalServerError("failed to serialize JWT"))?;

				// We need to update the request context with the new session
				if update_context {
					let auth_data = AuthData::from_session_and_user(global, session.clone(), &user).await?;
					request_context.set_auth(auth_data).await;
				}

				Ok(Some(TwoFaRequestFulfillResponse::Login(Session {
					id: session.id.0.into(),
					token,
					user_id: session.user_id.0.into(),
					expires_at: session.expires_at.into(),
					last_used_at: session.last_used_at.into(),
				})))
			}
			Some(Action::ChangePassword(action)) => {
				action.execute(global, user.id).await?;
				Ok(None)
			}
			None => Err(GqlError::InternalServerError("invalid two-factor authentication request").into()),
		}
	}

	/// Login with a session token. If via websocket this will authenticate the
	/// websocket connection.
	async fn login_with_token<'ctx>(
		&self,
		ctx: &Context<'_>,
		#[graphql(desc = "The JWT Session Token")] session_token: String,
		#[graphql(desc = "Setting this to false will make it so logging in does not authenticate the connection.")]
		update_context: Option<bool>,
	) -> Result<Session> {
		let global = ctx.get_global::<G>();
		let request_context = ctx.get_req_context();

		let jwt = AuthJwtPayload::verify(global, &session_token).map_err_gql(GqlError::InvalidInput {
			fields: vec!["sessionToken"],
			message: "invalid session token",
		})?;

		// TODO: maybe look to batch this
		let session: database::Session = sqlx::query_as(
			r#"
				UPDATE
					user_sessions
				SET
					last_used_at = NOW()
				WHERE
					id = $1
				RETURNING
					*
				"#,
		)
		.bind(Ulid::from(jwt.session_id))
		.fetch_optional(global.db().as_ref())
		.await?
		.map_err_gql(GqlError::InvalidInput {
			fields: vec!["sessionToken"],
			message: "invalid session token",
		})?;

		if !session.is_valid() {
			return Err(GqlError::Auth(AuthError::InvalidToken).into());
		}

		// We need to update the request context with the new session
		if update_context.unwrap_or(true) {
			let auth_data = AuthData::from_session(global, session.clone()).await?;
			request_context.set_auth(auth_data).await;
		}

		Ok(Session {
			id: session.id.0.into(),
			token: session_token,
			user_id: session.user_id.0.into(),
			expires_at: session.expires_at.into(),
			last_used_at: session.last_used_at.into(),
		})
	}

	/// If successful will return a new session for the account which just got
	/// created.
	#[allow(clippy::too_many_arguments)]
	async fn register<'ctx>(
		&self,
		ctx: &Context<'_>,
		#[graphql(desc = "The username of the user.", validator(custom = "UsernameValidator"))] username: String,
		#[graphql(desc = "The password of the user.", validator(custom = "PasswordValidator"))] password: String,
		#[graphql(desc = "The email of the user.", validator(email))] email: String,
		#[graphql(desc = "The captcha token from cloudflare turnstile.")] captcha_token: String,
		#[graphql(desc = "The validity of the session in seconds.")] validity: Option<u32>,
		#[graphql(desc = "Setting this to false will make it so logging in does not authenticate the connection.")]
		update_context: Option<bool>,
	) -> Result<Session> {
		let global = ctx.get_global::<G>();
		let request_context = ctx.get_req_context();

		if !validate_turnstile_token(global, &captcha_token)
			.await
			.map_err_gql("failed to validate captcha token")?
		{
			return Err(GqlError::InvalidInput {
				fields: vec!["captchaToken"],
				message: "capcha token is invalid",
			}
			.into());
		}

		let display_name = username.clone();
		let username = username.to_lowercase();
		let email = email.to_lowercase();

		if global
			.user_by_username_loader()
			.load(username.clone())
			.await
			.map_err_ignored_gql("failed to fetch user")?
			.is_some()
		{
			return Err(GqlError::InvalidInput {
				fields: vec!["username"],
				message: "username already taken",
			}
			.into());
		}

		// TODO: Create a video room
		let res = global
			.video_room_client()
			.clone()
			.create(pb::scuffle::video::v1::RoomCreateRequest {
				transcoding_config_id: None,
				recording_config_id: None,
				visibility: pb::scuffle::video::v1::types::Visibility::Public as i32,
				tags: Some(pb::scuffle::video::v1::types::Tags { tags: HashMap::new() }),
			})
			.await
			.map_err_ignored_gql("failed to create room")?;
		let channel_room_id = res
			.into_inner()
			.room
			.map_err_gql("failed to create room")?
			.id
			.map_err_gql("failed to create room")?
			.into_ulid();

		let mut tx = global.db().begin().await?;

		// TODO: maybe look to batch this
		let user: database::User = sqlx::query_as(
			r#"
			INSERT INTO users (
				id,
				username,
				display_name,
				display_color,
				password_hash,
				email,
				channel_room_id
			) VALUES (
				$1,
				$2,
				$3,
				$4,
				$5,
				$6,
				$7
			) RETURNING *
			"#,
		)
		.bind(Ulid::from(ulid::Ulid::new()))
		.bind(username)
		.bind(display_name)
		.bind(database::User::generate_display_color())
		.bind(database::User::hash_password(&password))
		.bind(email)
		.bind(Ulid::from(channel_room_id))
		.fetch_one(&mut *tx)
		.await?;

		let login_duration = validity.unwrap_or(60 * 60 * 24 * 7); // 7 days
		let expires_at = Utc::now() + Duration::seconds(login_duration as i64);

		// TODO: maybe look to batch this
		let session: database::Session = sqlx::query_as(
			r#"
			INSERT INTO user_sessions (
				id,
				user_id,
				expires_at
			) VALUES (
				$1,
				$2,
				$3
			) RETURNING *
			"#,
		)
		.bind(Ulid::from(ulid::Ulid::new()))
		.bind(user.id)
		.bind(expires_at)
		.fetch_one(&mut *tx)
		.await?;

		let jwt = AuthJwtPayload::from(session.clone());

		let token = jwt.serialize(global).map_err_gql("failed to serialize JWT")?;

		tx.commit().await?;

		// We need to update the request context with the new session
		if update_context.unwrap_or(true) {
			let global_state = global
				.global_state_loader()
				.load(())
				.await
				.map_err_ignored_gql("failed to fetch global state")?
				.map_err_gql("global state not found")?;
			// default is no roles and default permissions
			let auth_data = AuthData {
				session: session.clone(),
				user_roles: vec![],
				user_permissions: global_state.default_permissions,
			};
			request_context.set_auth(auth_data).await;
		}

		Ok(Session {
			id: session.id.0.into(),
			token,
			user_id: session.user_id.0.into(),
			expires_at: session.expires_at.into(),
			last_used_at: session.last_used_at.into(),
		})
	}

	/// Logout the user with the given session token. This will invalidate the
	/// session token.
	async fn logout<'ctx>(
		&self,
		ctx: &Context<'_>,
		#[graphql(
			desc = "You can provide a session token to logout of, if not provided the session will logout of the currently authenticated session."
		)]
		session_token: Option<String>,
	) -> Result<bool> {
		let global = ctx.get_global::<G>();
		let request_context = ctx.get_req_context();

		let session_id = if let Some(token) = &session_token {
			let jwt = AuthJwtPayload::verify(global, token).map_err_gql(GqlError::InvalidInput {
				fields: vec!["sessionToken"],
				message: "invalid session token",
			})?;
			jwt.session_id
		} else {
			request_context
				.auth(global)
				.await?
				.map_err_gql(GqlError::Auth(AuthError::NotLoggedIn))?
				.session
				.id
				.0
		};

		// TODO: maybe look to batch this
		sqlx::query(
			r#"
			DELETE FROM
				user_sessions
			WHERE
				id = $1
			"#,
		)
		.bind(Ulid::from(session_id))
		.execute(global.db().as_ref())
		.await?;

		if session_token.is_none() {
			request_context.reset_auth().await;
		}

		Ok(true)
	}
}
