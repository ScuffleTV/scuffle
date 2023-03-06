use super::error::{GqlError, Result, ResultExt};
use super::models::session::Session;
use super::GqlContext;
use crate::api::v1::jwt::JwtState;
use crate::global::GlobalState;
use async_graphql::{Context, Object};
use chrono::{Duration, Utc};
use common::types::{session, user};
use std::sync::Arc;

#[derive(Default, Clone)]
pub struct AuthMutation;

#[Object]
/// The mutation object for authentication
impl AuthMutation {
    /// Login using a username and password. If via websocket this will authenticate the websocket connection.
    async fn login<'ctx>(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The username of the user.")] username: String,
        #[graphql(desc = "The password of the user.")] password: String,
        #[graphql(desc = "The captcha token from cloudflare turnstile.")] captcha_token: String,
        #[graphql(
            desc = "The duration of the session in seconds. If not specified it will be 7 days."
        )]
        validity: Option<u32>,
    ) -> Result<Session> {
        let global = ctx
            .data::<Arc<GlobalState>>()
            .expect("failed to get global state");

        let request_context = ctx
            .data::<Arc<GqlContext>>()
            .expect("failed to get request context");

        if request_context.session.load().is_some() {
            return Err(GqlError::InvalidInput.with_message("Already logged in"));
        }

        if !global
            .validate_turnstile_token(&captcha_token)
            .await
            .map_err_gql("Failed to validate captcha token")?
        {
            return Err(GqlError::InvalidInput
                .with_message("Captcha token is not valid")
                .with_field(vec!["captchaToken"]));
        }

        let user = global
            .user_by_username_loader
            .load_one(username.to_lowercase())
            .await
            .map_err_gql("Failed to fetch user")?
            .ok_or(
                GqlError::InvalidInput
                    .with_message("Invalid username or password")
                    .with_field(vec!["username", "password"]),
            )?;

        if !user.verify_password(&password) {
            return Err(GqlError::InvalidInput
                .with_message("Invalid username or password")
                .with_field(vec!["username", "password"]));
        }

        let login_duration = validity.unwrap_or(60 * 60 * 24 * 7); // 7 days
        let expires_at = Utc::now() + Duration::seconds(login_duration as i64);

        // TODO: maybe look to batch this
        let session = sqlx::query_as!(
            session::Model,
            "INSERT INTO sessions (user_id, expires_at) VALUES ($1, $2) RETURNING *",
            user.id,
            expires_at,
        )
        .fetch_one(&*global.db)
        .await
        .map_err_gql("Failed to create session")?;

        let jwt = JwtState::from(session.clone());

        let token = jwt
            .serialize(global)
            .ok_or((GqlError::InternalServerError, "Failed to serialize JWT"))?;

        // We need to update the request context with the new session
        request_context
            .session
            .store(Arc::new(Some(session.clone())));

        Ok(Session {
            id: session.id,
            token,
            user_id: session.user_id,
            expires_at: session.expires_at.into(),
            last_used_at: session.last_used_at.into(),
            created_at: session.created_at.into(),
        })
    }

    /// Login with a session token. If via websocket this will authenticate the websocket connection.
    async fn login_with_token<'ctx>(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The JWT Session Token")] session_token: String,
        #[graphql(
            desc = "Setting this to false will make it so logging in does not authenticate the websocket connection."
        )]
        update_context: Option<bool>,
    ) -> Result<Session> {
        let global = ctx
            .data::<Arc<GlobalState>>()
            .expect("failed to get global state");

        let request_context = ctx
            .data::<Arc<GqlContext>>()
            .expect("failed to get request context");

        if request_context.get_session(global).await?.is_some() {
            return Err(GqlError::InvalidInput.with_message("Already logged in"));
        }

        let jwt = JwtState::verify(global, &session_token).ok_or(
            GqlError::InvalidInput
                .with_message("Invalid session token")
                .with_field(vec!["sessionToken"]),
        )?;

        // TODO: maybe look to batch this
        let session = sqlx::query_as!(
            session::Model,
            "UPDATE sessions SET last_used_at = NOW() WHERE id = $1 RETURNING *",
            jwt.session_id,
        )
        .fetch_optional(&*global.db)
        .await
        .map_err_gql("failed to fetch session")?
        .ok_or(
            GqlError::InvalidInput
                .with_message("Invalid session token")
                .with_field(vec!["sessionToken"]),
        )?;

        if !session.is_valid() {
            return Err(GqlError::InvalidSession.with_message("Session token is no longer valid"));
        }

        // We need to update the request context with the new session
        if update_context.unwrap_or(true) {
            request_context
                .session
                .store(Arc::new(Some(session.clone())));
        }

        Ok(Session {
            id: session.id,
            token: session_token,
            user_id: session.user_id,
            expires_at: session.expires_at.into(),
            last_used_at: session.last_used_at.into(),
            created_at: session.created_at.into(),
        })
    }

    /// If successful will return a new session for the account which just got created.
    async fn register<'ctx>(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The username of the user.")] username: String,
        #[graphql(desc = "The password of the user.")] password: String,
        #[graphql(desc = "The email of the user.")] email: String,
        #[graphql(desc = "The captcha token from cloudflare turnstile.")] captcha_token: String,
        #[graphql(desc = "The validity of the session in seconds.")] validity: Option<u32>,
    ) -> Result<Session> {
        let global = ctx
            .data::<Arc<GlobalState>>()
            .expect("failed to get global state");

        let request_context = ctx
            .data::<Arc<GqlContext>>()
            .expect("failed to get request context");

        if request_context.get_session(global).await?.is_some() {
            return Err(GqlError::InvalidInput.with_message("Already logged in"));
        }

        if !global
            .validate_turnstile_token(&captcha_token)
            .await
            .map_err_gql("Failed to validate captcha token")?
        {
            return Err(GqlError::InvalidInput
                .with_message("Capcha token is invalid")
                .with_field(vec!["captchaToken"]));
        }

        let username = username.to_lowercase();
        let email = email.to_lowercase();

        user::validate_username(&username).map_err(|e| {
            GqlError::InvalidInput
                .with_message(e)
                .with_field(vec!["username"])
        })?;
        user::validate_password(&password).map_err(|e| {
            GqlError::InvalidInput
                .with_message(e)
                .with_field(vec!["password"])
        })?;
        user::validate_email(&email).map_err(|e| {
            GqlError::InvalidInput
                .with_message(e)
                .with_field(vec!["email"])
        })?;

        if global
            .user_by_username_loader
            .load_one(username.clone())
            .await
            .map_err_gql("failed to fetch user")?
            .is_some()
        {
            return Err(GqlError::InvalidInput
                .with_message("Username already taken")
                .with_field(vec!["username"]));
        }

        let mut tx = global
            .db
            .begin()
            .await
            .map_err_gql("Failed to create user")?;

        // TODO: maybe look to batch this
        let user_id = sqlx::query!(
            "INSERT INTO users (username, password_hash, email) VALUES ($1, $2, $3) RETURNING id",
            username,
            user::hash_password(&password),
            email,
        )
        .map(|row| row.id)
        .fetch_one(&mut tx)
        .await
        .map_err_gql("Failed to create user")?;

        let login_duration = validity.unwrap_or(60 * 60 * 24 * 7); // 7 days
        let expires_at = Utc::now() + Duration::seconds(login_duration as i64);

        // TODO: maybe look to batch this
        let session = sqlx::query_as!(
            session::Model,
            "INSERT INTO sessions (user_id, expires_at) VALUES ($1, $2) RETURNING *",
            user_id,
            expires_at,
        )
        .fetch_one(&mut tx)
        .await
        .map_err_gql("Failed to create session")?;

        let jwt = JwtState::from(session.clone());

        let token = jwt
            .serialize(global)
            .ok_or((GqlError::InternalServerError, "Failed to serialize JWT"))?;

        tx.commit()
            .await
            .map_err_gql("Failed to commit transaction")?;

        request_context
            .session
            .store(Arc::new(Some(session.clone())));

        Ok(Session {
            id: session.id,
            token,
            user_id: session.user_id,
            expires_at: session.expires_at.into(),
            last_used_at: session.last_used_at.into(),
            created_at: session.created_at.into(),
        })
    }

    /// Logout the user with the given session token. This will invalidate the session token.
    async fn logout<'ctx>(
        &self,
        ctx: &Context<'_>,
        #[graphql(
            desc = "You can provide a session token to logout of, if not provided the session will logout of the currently authenticated session."
        )]
        session_token: Option<String>,
    ) -> Result<bool> {
        let global = ctx
            .data::<Arc<GlobalState>>()
            .expect("failed to get global state");

        let request_context = ctx
            .data::<Arc<GqlContext>>()
            .expect("failed to get request context");
        let session = if session_token.is_none() {
            request_context.get_session(global).await?
        } else {
            None
        };

        let jwt = session_token.and_then(|token| JwtState::verify(global, &token));

        let session_id = match (
            Option::as_ref(&jwt).map(|jwt| jwt.session_id),
            Option::as_ref(&session).map(|s| s.id),
        ) {
            (Some(id), _) => id,
            (None, Some(id)) => id,
            (None, None) => {
                return Err(GqlError::InvalidInput
                    .with_message("Not logged in")
                    .with_field(vec!["sessionToken"]));
            }
        };

        // TODO: maybe look to batch this
        sqlx::query!(
            "UPDATE sessions SET invalidated_at = NOW() WHERE id = $1",
            session_id
        )
        .execute(&*global.db)
        .await
        .map_err_gql("Failed to update session")?;

        if jwt.is_none() {
            request_context.session.store(Arc::new(None));
        }

        Ok(true)
    }
}
