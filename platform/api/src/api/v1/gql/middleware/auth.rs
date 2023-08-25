use std::sync::Arc;

use async_graphql::ErrorExtensions;
use hyper::http::header;
use hyper::{Body, Request, Response, StatusCode};
use routerify::{prelude::RequestExt, Middleware};
use serde_json::json;

use crate::api::error::RouteError;
use crate::api::ext::RequestExt as _;
use crate::api::v1::gql::error::{GqlError, GqlErrorInterface, ResultExt};
use crate::api::v1::jwt::JwtState;
use crate::api::v1::request_context::{AuthData, RequestContext};
use crate::global::GlobalState;

pub fn auth_middleware(_: &Arc<GlobalState>) -> Middleware<Body, RouteError> {
    Middleware::pre(|req| async move {
        authenticate(req).await.map_err(|e| {
            let gql_err = e.extend();
            let resp = Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(
                    json!({
                        "data": null,
                        "errors": vec![gql_err],
                        "extensions": null,
                    })
                    .to_string(),
                ))
                .expect("failed to build response");
            Into::<RouteError>::into(resp)
        })
    })
}

async fn authenticate(req: Request<Body>) -> Result<Request<Body>, GqlErrorInterface> {
    let context = RequestContext::default();
    req.set_context(context.clone());

    let Some(token) = req.headers().get(header::AUTHORIZATION) else {
        // No Authorization header
        return Ok(req);
    };

    let global = req.get_global().map_err_gql("failed to get global state")?;

    let Ok(token) = token.to_str() else {
        return Err(GqlError::InvalidSession.with_message("token must be an ascii string"));
    };

    // Token's will start with "Bearer " so we need to remove that
    let Some(token) = token.strip_prefix("Bearer ") else {
        return Err(GqlError::InvalidSession.with_message("token must be a bearer token"));
    };

    let Some(jwt) = JwtState::verify(&global, token) else {
        return Err(GqlError::InvalidSession.with_message("invalid token"));
    };

    let Some(session) = global
        .session_by_id_loader
        .load_one(jwt.session_id)
        .await
        .map_err_gql("failed to fetch session")?
    else {
        return Err(GqlError::InvalidSession.with_message("invalid token"));
    };

    if !session.is_valid() {
        return Err(GqlError::InvalidSession.with_message("invalid session"));
    }

    let data = AuthData::from_session_id(&global, session.id)
        .await
        .map_err(|e| GqlError::InvalidSession.with_message(e))?;

    context.set_auth(data).await;

    Ok(req)
}
