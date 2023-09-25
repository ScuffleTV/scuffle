use std::sync::Arc;

use hyper::http::header;
use routerify::{prelude::RequestExt, Middleware};

use crate::api::error::{ApiErrorInterface, ResultExt};
use crate::api::ext::RequestExt as _;
use crate::api::v1::jwt::JwtState;
use crate::api::v1::request_context::{AuthData, RequestContext};
use crate::global::GlobalState;

#[derive(thiserror::Error, Debug, Clone)]
pub enum AuthError {
    #[error("token must be ascii only")]
    HeaderToStr,
    #[error("token must be a bearer token")]
    NotBearerToken,
    /// The user is not logged in
    #[error("not logged in")]
    NotLoggedIn,
    #[error("invalid token")]
    InvalidToken,
    #[error("unsolved two factor authentication challenge")]
    UnsolvedTwoFaChallenge,
}

pub fn auth_middleware(_: &Arc<GlobalState>) -> Middleware<hyper::Body, ApiErrorInterface> {
    Middleware::pre(|req| async move {
        let context = RequestContext::default();
        req.set_context(context.clone());

        let Some(token) = req.headers().get(header::AUTHORIZATION) else {
            // No Authorization header
            return Ok(req);
        };

        let global = req.get_global()?;

        let token = token
            .to_str()
            .map_err(|_| AuthError::HeaderToStr)?
            .strip_prefix("Bearer ") // Tokens will start with "Bearer " so we need to remove that
            .ok_or(AuthError::NotBearerToken)?;

        let jwt = JwtState::verify(&global, token).ok_or(AuthError::InvalidToken)?;

        let session = global
            .session_by_id_loader
            .load(jwt.session_id)
            .await
            .ok()
            .map_err_route("failed to fetch session")?
            .ok_or(AuthError::InvalidToken)?;

        if !session.is_valid() {
            return Err(AuthError::InvalidToken.into());
        }

        let data = AuthData::from_session(&global, session)
            .await
            .map_err(|_| AuthError::InvalidToken)?;

        context.set_auth(data).await;

        Ok(req)
    })
}
