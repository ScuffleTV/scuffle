use std::sync::{Arc, Weak};

use hyper::http::header;
use hyper::{Body, StatusCode};
use routerify::{prelude::RequestExt, Middleware};

use crate::api::error::{ResultExt, RouteError};
use crate::api::v1::jwt::JwtState;
use crate::global::GlobalState;

pub fn auth_middleware(_global: &Arc<GlobalState>) -> Middleware<Body, RouteError> {
    Middleware::pre(|req| async move {
        let Some(token) = req.headers().get(header::AUTHORIZATION) else {
            return Ok(req);
        };

        let global = req
            .data::<Weak<GlobalState>>()
            .expect("Global state not found")
            .upgrade()
            .ok_or((
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to upgrade global state",
            ))?;
        let token = token
            .to_str()
            .map_err(|e| (StatusCode::UNAUTHORIZED, "invalid authentication token", e))?;

        // Token's will start with "Bearer " so we need to remove that
        if !token.starts_with("Bearer ") {
            return Err(RouteError::from((
                StatusCode::UNAUTHORIZED,
                "invalid authentication token",
            )));
        }

        let jwt = JwtState::verify(&global, token.trim_start_matches("Bearer "))
            .ok_or((StatusCode::UNAUTHORIZED, "invalid authentication token"))?;

        let session = global
            .session_by_id_loader
            .load_one(jwt.session_id)
            .await
            .extend_route("failed to fetch session")?
            .ok_or((StatusCode::UNAUTHORIZED, "invalid authentication token"))?;

        if !session.validate() {
            return Err(RouteError::from((
                StatusCode::UNAUTHORIZED,
                "session token has been invalidated",
            )));
        }

        req.set_context(session);

        Ok(req)
    })
}
