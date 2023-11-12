use std::sync::Arc;

use common::http::{ext::*, RouteError};
use hyper::http::header;
use hyper::StatusCode;
use routerify::{prelude::RequestExt as _, Middleware};

use crate::api::error::ApiError;
use crate::api::jwt::JwtState;
use crate::api::request_context::{AuthData, RequestContext};
use crate::global::ApiGlobal;

#[derive(thiserror::Error, Debug, Clone)]
pub enum AuthError {
    #[error("token must be ascii only")]
    HeaderToStr,
    #[error("token must be a bearer token")]
    NotBearerToken,
    #[error("not logged in")]
    NotLoggedIn,
    #[error("invalid token")]
    InvalidToken,
    #[error("session expired")]
    SessionExpired,
}

pub fn auth_middleware<G: ApiGlobal>(_: &Arc<G>) -> Middleware<hyper::Body, RouteError<ApiError>> {
    Middleware::pre(|req| async move {
        let context = RequestContext::default();
        req.set_context(context.clone());

        let Some(token) = req.headers().get(header::AUTHORIZATION) else {
            // No Authorization header
            return Ok(req);
        };

        let global = req.get_global::<G>()?;

        let token = token
            .to_str()
            .map_ignore_err_route((StatusCode::BAD_REQUEST, "invalid token"))?
            .strip_prefix("Bearer ") // Tokens will start with "Bearer " so we need to remove that
            .map_err_route((StatusCode::BAD_REQUEST, "invalid token"))?;

        let jwt = JwtState::verify(&global, token)
            .map_err_route((StatusCode::UNAUTHORIZED, "invalid token"))?;

        let session = global
            .session_by_id_loader()
            .load(jwt.session_id)
            .await
            .map_ignore_err_route("failed to fetch session")?
            .map_err_route((StatusCode::UNAUTHORIZED, "invalid token"))?;

        if !session.is_valid() {
            return Err((StatusCode::UNAUTHORIZED, "invalid token").into());
        }

        let data = AuthData::from_session(&global, session)
            .await
            .map_ignore_err_route("failed to create auth data")?;

        context.set_auth(data).await;

        Ok(req)
    })
}
