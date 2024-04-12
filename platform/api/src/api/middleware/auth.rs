use std::sync::Arc;

use binary_helper::global::RequestGlobalExt;
use hyper::body::Incoming;
use hyper::http::header;
use utils::http::ext::*;
use utils::http::router::ext::RequestExt;
use utils::http::router::middleware::{middleware_fn, Middleware};
use utils::http::RouteError;

use crate::api::auth::{AuthData, AuthError};
use crate::api::error::ApiError;
use crate::api::jwt::{AuthJwtPayload, JwtState};
use crate::api::request_context::RequestContext;
use crate::api::Body;
use crate::global::ApiGlobal;

pub fn auth_middleware<G: ApiGlobal>(_: &Arc<G>) -> impl Middleware<Incoming, Body, RouteError<ApiError>> {
	middleware_fn(|mut req, next| async move {
		let context = RequestContext::default();
		req.provide(context.clone());

		let Some(token) = req.headers().get(header::AUTHORIZATION) else {
			// No Authorization header
			return next(req).await;
		};

		let global = req.get_global::<G, _>()?;

		let token = token
			.to_str()
			.map_err(|_| AuthError::InvalidToken)
			.into_err_route()?
			.strip_prefix("Bearer ") // Tokens will start with "Bearer " so we need to remove that
			.ok_or(AuthError::InvalidToken)
			.into_err_route()?;

		let jwt = AuthJwtPayload::verify(&global, token)
			.ok_or(AuthError::InvalidToken)
			.into_err_route()?;

		let data = AuthData::from_session_id(&global, jwt.session_id).await.into_err_route()?;

		context.set_auth(data).await;

		next(req).await
	})
}
