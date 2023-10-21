use crate::api::middleware::auth::AuthError;
use crate::api::v1::gql::validators::{PasswordValidator, UsernameValidator};
use crate::api::v1::jwt::JwtState;
use crate::api::v1::request_context::AuthData;
use crate::{
    api::v1::gql::{
        error::{GqlError, Result, ResultExt},
        ext::ContextExt,
        models::session::Session,
    },
    database,
};
use async_graphql::{Context, Object};
use chrono::{Duration, Utc};
use ulid::Ulid;
use uuid::Uuid;

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
            .map_err_gql("failed to validate captcha token")?
        {
            return Err(GqlError::InvalidInput {
                fields: vec!["captchaToken"],
                message: "capcha token is invalid",
            }
            .into());
        }

        let user = global
            .user_by_username_loader
            .load(username.to_lowercase())
            .await
            .map_err_gql("failed to fetch user")?
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

        let login_duration = validity.unwrap_or(60 * 60 * 24 * 7); // 7 days
        let expires_at = Utc::now() + Duration::seconds(login_duration as i64);

        // TODO: maybe look to batch this
        let mut tx = global.db.begin().await?;

        let session: database::Session = sqlx::query_as(
            "INSERT INTO user_sessions (id, user_id, two_fa_solved, expires_at) VALUES ($1, $2, $3, $4) RETURNING *",
        )
        .bind(Uuid::from(Ulid::new()))
        .bind(user.id)
        .bind(!user.totp_enabled)
        .bind(expires_at)
        .fetch_one(tx.as_mut())
        .await?;

        sqlx::query("UPDATE users SET last_login_at = NOW() WHERE id = $1")
            .bind(user.id)
            .execute(tx.as_mut())
            .await?;

        tx.commit().await?;

        let jwt = JwtState::from(session.clone());

        let token = jwt
            .serialize(global)
            .ok_or(GqlError::InternalServerError("failed to serialize JWT"))?;

        // We need to update the request context with the new session
        if update_context.unwrap_or(true) {
            let auth_data = AuthData::from_session_and_user(global, session.clone(), &user)
                .await
                .map_err(GqlError::InternalServerError)?;
            request_context.set_auth(auth_data).await;
        }

        Ok(Session {
            id: session.id.0.into(),
            token,
            user_id: session.user_id.0.into(),
            two_fa_solved: session.two_fa_solved,
            expires_at: session.expires_at.into(),
            last_used_at: session.last_used_at.into(),
        })
    }

    /// Verify a TOTP code for the currently authenticated user.
    async fn verify_totp_code(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The TOTP code to verify.")] code: String,
    ) -> Result<Session> {
        let global = ctx.get_global();
        let request_context = ctx.get_req_context();

        // Notice that we're using `auth_unchecked` here, because the 2fa challenge isn't solved yet
        let mut auth = request_context
            .auth_unchecked()
            .await
            .ok_or(GqlError::Auth(AuthError::NotLoggedIn))?;
        // Still need to check if the session is valid
        if !auth.session.is_valid() {
            return Err(GqlError::Auth(AuthError::InvalidToken).into());
        }

        let user = global
            .user_by_id_loader
            .load(auth.session.user_id.into())
            .await
            .ok()
            .map_err_gql("failed to fetch user")?
            .ok_or(GqlError::NotFound("user"))?;

        if !user.verify_totp_code(&code, true)? {
            return Err(GqlError::InvalidInput {
                fields: vec!["code"],
                message: "wrong code",
            }
            .into());
        }

        let session: database::Session = sqlx::query_as(
            "UPDATE user_sessions SET two_fa_solved = true WHERE id = $1 RETURNING *",
        )
        .bind(auth.session.id)
        .fetch_one(global.db.as_ref())
        .await?;
        auth.session = session.clone();
        request_context.set_auth(auth).await;

        let jwt = JwtState::from(session.clone());
        let token = jwt
            .serialize(global)
            .ok_or(GqlError::InternalServerError("failed to serialize JWT"))?;

        Ok(Session {
            id: session.id.0.into(),
            token,
            user_id: session.user_id.0.into(),
            two_fa_solved: session.two_fa_solved,
            expires_at: session.expires_at.into(),
            last_used_at: session.last_used_at.into(),
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

        let jwt = JwtState::verify(global, &session_token).map_err_gql(GqlError::InvalidInput {
            fields: vec!["sessionToken"],
            message: "invalid session token",
        })?;

        // TODO: maybe look to batch this
        let session: database::Session = sqlx::query_as(
            "UPDATE user_sessions SET last_used_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(Uuid::from(jwt.session_id))
        .fetch_optional(global.db.as_ref())
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
            let auth_data = AuthData::from_session(global, session.clone())
                .await
                .map_err(GqlError::InternalServerError)?;
            request_context.set_auth(auth_data).await;
        }

        Ok(Session {
            id: session.id.0.into(),
            token: session_token,
            user_id: session.user_id.0.into(),
            two_fa_solved: session.two_fa_solved,
            expires_at: session.expires_at.into(),
            last_used_at: session.last_used_at.into(),
        })
    }

    /// If successful will return a new session for the account which just got created.
    #[allow(clippy::too_many_arguments)]
    async fn register<'ctx>(
        &self,
        ctx: &Context<'_>,
        #[graphql(
            desc = "The username of the user.",
            validator(custom = "UsernameValidator")
        )]
        username: String,
        #[graphql(
            desc = "The password of the user.",
            validator(custom = "PasswordValidator")
        )]
        password: String,
        #[graphql(desc = "The email of the user.", validator(email))] email: String,
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
            .user_by_username_loader
            .load(username.clone())
            .await
            .map_err_gql("failed to fetch user")?
            .is_some()
        {
            return Err(GqlError::InvalidInput {
                fields: vec!["username"],
                message: "username already taken",
            }
            .into());
        }

        let mut tx = global.db.begin().await?;

        // TODO: maybe look to batch this
        let user: database::User = sqlx::query_as("INSERT INTO users (id, username, display_name, display_color, password_hash, email) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *")
            .bind(Uuid::from(Ulid::new()))
            .bind(username)
            .bind(display_name)
            .bind(database::User::generate_display_color())
            .bind(database::User::hash_password(&password))
            .bind(email)
            .fetch_one(&mut *tx)
            .await?;

        let login_duration = validity.unwrap_or(60 * 60 * 24 * 7); // 7 days
        let expires_at = Utc::now() + Duration::seconds(login_duration as i64);

        // TODO: maybe look to batch this
        let session: database::Session = sqlx::query_as(
            "INSERT INTO user_sessions (id, user_id, two_fa_solved, expires_at) VALUES ($1, $2, $3, $4) RETURNING *",
        )
        .bind(Uuid::from(Ulid::new()))
        .bind(user.id)
        .bind(!user.totp_enabled)
        .bind(expires_at)
        .fetch_one(&mut *tx)
        .await?;

        let jwt = JwtState::from(session.clone());

        let token = jwt
            .serialize(global)
            .map_err_gql("failed to serialize JWT")?;

        tx.commit().await?;

        // We need to update the request context with the new session
        if update_context.unwrap_or(true) {
            let global_state = global
                .global_state_loader
                .load(())
                .await
                .map_err_gql("failed to fetch global state")?
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
            two_fa_solved: session.two_fa_solved,
            expires_at: session.expires_at.into(),
            last_used_at: session.last_used_at.into(),
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
            let jwt = JwtState::verify(global, token).map_err_gql(GqlError::InvalidInput {
                fields: vec!["sessionToken"],
                message: "invalid session token",
            })?;
            jwt.session_id
        } else {
            request_context
                .auth()
                .await?
                .map_err_gql(GqlError::Auth(AuthError::NotLoggedIn))?
                .session
                .id
                .0
        };

        // TODO: maybe look to batch this
        sqlx::query("DELETE FROM user_sessions WHERE id = $1")
            .bind(Uuid::from(session_id))
            .execute(global.db.as_ref())
            .await?;

        if session_token.is_none() {
            request_context.reset_auth().await;
        }

        Ok(true)
    }
}
