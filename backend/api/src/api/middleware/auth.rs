use std::sync::Arc;

use hyper::http::header;
use hyper::{header::HeaderValue, Body, StatusCode};
use routerify::{prelude::RequestExt, Middleware};

use crate::api::error::{ResultExt, RouteError};
use crate::api::ext::RequestExt as _;
use crate::api::middleware::response_headers::RequestExt as _;
use crate::api::v1::jwt::JwtState;
use crate::global::GlobalState;

const X_AUTH_TOKEN_CHECK: &str = "X-Auth-Token-Check";
const X_AUTH_TOKEN_CHECK_STATUS: &str = "X-Auth-Token-Check-Status";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthTokenCheck {
    Always,
    WhenRequired,
}

impl From<AuthTokenCheck> for HeaderValue {
    fn from(mode: AuthTokenCheck) -> Self {
        match mode {
            AuthTokenCheck::Always => HeaderValue::from_static("always"),
            AuthTokenCheck::WhenRequired => HeaderValue::from_static("when-required"),
        }
    }
}

impl From<&HeaderValue> for AuthTokenCheck {
    fn from(value: &HeaderValue) -> Self {
        match value.to_str().unwrap_or_default() {
            "always" => Self::Always,
            "when-required" => Self::WhenRequired,
            _ => Self::WhenRequired,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthTokenCheckStatus {
    Success,
    Failed,
}

impl From<AuthTokenCheckStatus> for HeaderValue {
    fn from(mode: AuthTokenCheckStatus) -> Self {
        match mode {
            AuthTokenCheckStatus::Success => HeaderValue::from_static("success"),
            AuthTokenCheckStatus::Failed => HeaderValue::from_static("failed"),
        }
    }
}

macro_rules! fail_fast {
    ($mode:ident, $req:ident) => {
        $req.set_response_header(X_AUTH_TOKEN_CHECK, $mode);
        $req.set_response_header(X_AUTH_TOKEN_CHECK_STATUS, AuthTokenCheckStatus::Failed);
        if $mode == AuthTokenCheck::Always {
            return Err(RouteError::from((StatusCode::UNAUTHORIZED, "unauthorized")));
        }
        return Ok($req);
    };
}

impl Default for AuthTokenCheck {
    fn default() -> Self {
        Self::WhenRequired
    }
}

pub fn auth_middleware(_: &Arc<GlobalState>) -> Middleware<Body, RouteError> {
    Middleware::pre(|req| async move {
        let mode = req
            .headers()
            .get(X_AUTH_TOKEN_CHECK)
            .map(AuthTokenCheck::from)
            .unwrap_or_default();

        let Some(token) = req.headers().get(header::AUTHORIZATION) else {
            fail_fast!(mode, req);
        };

        let global = req.get_global()?;
        let Ok(token) = token.to_str() else {
            fail_fast!(mode, req);
        };

        // Token's will start with "Bearer " so we need to remove that
        let Some(token) = token.strip_prefix("Bearer ") else {
            fail_fast!(mode, req);
        };

        let Some(jwt) = JwtState::verify(&global, token) else {
            fail_fast!(mode, req);
        };

        let Some(session) = global
            .session_by_id_loader
            .load_one(jwt.session_id)
            .await
            .map_err_route("failed to fetch session")?
        else {
            fail_fast!(mode, req);
        };

        if !session.is_valid() {
            fail_fast!(mode, req);
        }

        let permissions = global
            .user_permisions_by_id_loader
            .load_one(session.user_id)
            .await
            .map_err_route("failed to fetch user permissions")?
            .unwrap_or_default();

        req.set_response_header(X_AUTH_TOKEN_CHECK_STATUS, AuthTokenCheckStatus::Success);
        req.set_context((session, permissions));

        Ok(req)
    })
}
