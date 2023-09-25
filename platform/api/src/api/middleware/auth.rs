use std::sync::Arc;

use hyper::http::header;
use routerify::{prelude::RequestExt, Middleware};

use crate::api::error::{ResultExt, ApiErrorInterface};
use crate::api::ext::RequestExt as _;
use crate::api::v1::jwt::JwtState;
use crate::api::v1::request_context::{AuthData, RequestContext};
use crate::global::GlobalState;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("token must be ascii only")]
    HeaderToStr(#[from] hyper::http::header::ToStrError),
    #[error("token must be a bearer token")]
    NotBearerToken,
    #[error("invalid token")]
    InvalidToken,
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
            .map_err(AuthError::HeaderToStr)?
            .strip_prefix("Bearer ") // Tokens will start with "Bearer " so we need to remove that
            .ok_or(AuthError::NotBearerToken)?;

        let jwt = JwtState::verify(&global, token).ok_or(AuthError::InvalidToken)?;

        let session = global
            .session_by_id_loader
            .load(jwt.session_id)
            .await
            .ok()
            .map_err_route("failed to fetch session")?
            .and_then(|s| s.is_valid().then_some(s))
            .ok_or(AuthError::InvalidToken)?;

        let data = AuthData::from_session(&global, session)
            .await
            .map_err(|_| AuthError::InvalidToken)?;

        context.set_auth(data).await;

        Ok(req)
    })
}
