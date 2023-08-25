use crate::api::v1::gql::{
    error::{GqlError, Result, ResultExt},
    ext::ContextExt,
    models::session::Session,
};
use crate::api::v1::jwt::JwtState;
use crate::api::v1::request_context::AuthData;
use crate::database::{session, user};
use async_graphql::{Context, Object};
use chrono::{Duration, Utc};
use ulid::Ulid;

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
        #[graphql(
            desc = "Setting this to false will make it so logging in does not authenticate the connection."
        )]
        update_context: Option<bool>,
    ) -> Result<Session> {
        let global = ctx.get_global();
        let request_context = ctx.get_req_context();

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
        let mut tx = global
            .db
            .begin()
            .await
            .map_err_gql("Failed to start transaction")?;

        let session: session::Model = sqlx::query_as(
            "INSERT INTO user_sessions (id, user_id, expires_at) VALUES (ulid_to_uuid($1), $2, $3) RETURNING *",
        )
        .bind(Ulid::new().to_string())
        .bind(user.id)
        .bind(expires_at)
        .fetch_one(&mut *tx)
        .await
        .map_err_gql("Failed to create session")?;

        sqlx::query("UPDATE users SET last_login_at = NOW() WHERE id = $1")
            .bind(user.id)
            .execute(&mut *tx)
            .await
            .map_err_gql("Failed to update user")?;

        tx.commit()
            .await
            .map_err_gql("Failed to commit transaction")?;

        let jwt = JwtState::from(session.clone());

        let token = jwt
            .serialize(global)
            .ok_or((GqlError::InternalServerError, "Failed to serialize JWT"))?;

        // We need to update the request context with the new session
        if update_context.unwrap_or(true) {
            let auth_data = AuthData::from_session_and_user(global, session.clone(), user.clone())
                .await
                .map_err(|e| GqlError::InternalServerError.with_message(e))?;
            request_context.set_auth(auth_data).await;
        }

        Ok(Session {
            id: session.id.into(),
            token,
            user_id: session.user_id.into(),
            expires_at: session.expires_at.into(),
            last_used_at: session.last_used_at.into(),
            _user: Some(user.into()),
        })
    }

    /// Login with a session token. If via websocket this will authenticate the websocket connection.
    async fn login_with_token<'ctx>(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The JWT Session Token")] session_token: String,
        #[graphql(
            desc = "Setting this to false will make it so logging in does not authenticate the connection."
        )]
        update_context: Option<bool>,
    ) -> Result<Session> {
        let global = ctx.get_global();
        let request_context = ctx.get_req_context();

        let jwt = JwtState::verify(global, &session_token).ok_or(
            GqlError::InvalidInput
                .with_message("Invalid session token")
                .with_field(vec!["sessionToken"]),
        )?;

        // TODO: maybe look to batch this
        let session: session::Model = sqlx::query_as(
            "UPDATE user_sessions SET last_used_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(jwt.session_id)
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
            let auth_data = AuthData::from_session(global, session.clone())
                .await
                .map_err(|e| GqlError::InternalServerError.with_message(e))?;
            request_context.set_auth(auth_data).await;
        }

        Ok(Session {
            id: session.id.into(),
            token: session_token,
            user_id: session.user_id.into(),
            expires_at: session.expires_at.into(),
            last_used_at: session.last_used_at.into(),
            _user: None,
        })
    }

    /// If successful will return a new session for the account which just got created.
    #[allow(clippy::too_many_arguments)]
    async fn register<'ctx>(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The username of the user.")] username: String,
        #[graphql(desc = "The password of the user.")] password: String,
        #[graphql(desc = "The email of the user.")] email: String,
        #[graphql(desc = "The captcha token from cloudflare turnstile.")] captcha_token: String,
        #[graphql(desc = "The validity of the session in seconds.")] validity: Option<u32>,
        #[graphql(
            desc = "Setting this to false will make it so logging in does not authenticate the connection."
        )]
        update_context: Option<bool>,
    ) -> Result<Session> {
        let global = ctx.get_global();
        let request_context = ctx.get_req_context();

        if !global
            .validate_turnstile_token(&captcha_token)
            .await
            .map_err_gql("Failed to validate captcha token")?
        {
            return Err(GqlError::InvalidInput
                .with_message("Capcha token is invalid")
                .with_field(vec!["captchaToken"]));
        }

        let display_name = username.clone();
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
        let user: user::Model = sqlx::query_as("INSERT INTO users (id, username, display_name, display_color, password_hash, email) VALUES (ulid_to_uuid($1), $2, $3, $4, $5, $6) RETURNING *")
            .bind(Ulid::new().to_string())
            .bind(username)
            .bind(display_name)
            .bind(user::generate_display_color())
            .bind(user::hash_password(&password))
            .bind(email)
            .fetch_one(&mut *tx)
            .await
            .map_err_gql("Failed to create user")?;

        let login_duration = validity.unwrap_or(60 * 60 * 24 * 7); // 7 days
        let expires_at = Utc::now() + Duration::seconds(login_duration as i64);

        // TODO: maybe look to batch this
        let session: session::Model = sqlx::query_as(
            "INSERT INTO user_sessions (id, user_id, expires_at) VALUES (ulid_to_uuid($1), $2, $3) RETURNING *",
        )
        .bind(Ulid::new().to_string())
        .bind(user.id)
        .bind(expires_at)
        .fetch_one(&mut *tx)
        .await
        .map_err_gql("Failed to create session")?;

        let jwt = JwtState::from(session.clone());

        let token = jwt
            .serialize(global)
            .ok_or((GqlError::InternalServerError, "Failed to serialize JWT"))?;

        tx.commit()
            .await
            .map_err_gql("Failed to commit transaction")?;

        // We need to update the request context with the new session
        if update_context.unwrap_or(true) {
            let global_state = global
                .global_state_loader
                .load_one(())
                .await
                .map_err_gql(
                    GqlError::InternalServerError.with_message("Failed to fetch global state"),
                )?
                .ok_or(
                    GqlError::InternalServerError.with_message("Failed to fetch global state"),
                )?;
            // default is no roles and default permissions
            let auth_data = AuthData {
                session: session.clone(),
                user_roles: vec![],
                user_permissions: global_state.default_permissions,
            };
            request_context.set_auth(auth_data).await;
        }

        Ok(Session {
            id: session.id.into(),
            token,
            user_id: session.user_id.into(),
            expires_at: session.expires_at.into(),
            last_used_at: session.last_used_at.into(),
            _user: Some(user.into()),
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
        let global = ctx.get_global();
        let request_context = ctx.get_req_context();

        let session_id = if let Some(token) = &session_token {
            let jwt = JwtState::verify(global, token).ok_or(
                GqlError::InvalidInput
                    .with_message("Invalid session token")
                    .with_field(vec!["sessionToken"]),
            )?;
            jwt.session_id
        } else {
            request_context
                .auth()
                .await
                .ok_or(GqlError::InvalidInput.with_message("Not logged in"))?
                .session
                .id
        };

        // TODO: maybe look to batch this
        sqlx::query("DELETE FROM user_sessions WHERE id = $1")
            .bind(session_id)
            .execute(&*global.db)
            .await
            .map_err_gql("Failed to update session")?;

        if session_token.is_none() {
            request_context.reset_auth().await;
        }

        Ok(true)
    }
}
