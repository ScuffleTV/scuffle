use std::collections::HashMap;
use std::sync::Arc;

use hyper::StatusCode;
use scuffle_utils::http::RouteError;
use ulid::Ulid;

use super::error::ApiError;
use crate::database::{Role, RolePermission, Session, User};
use crate::global::ApiGlobal;

#[derive(thiserror::Error, Debug, Clone)]
pub enum AuthError {
	#[error("not logged in")]
	NotLoggedIn,
	#[error("invalid token")]
	InvalidToken,
	#[error("session expired")]
	SessionExpired,
	#[error("failed to fetch global state")]
	FetchGlobalState,
	#[error("failed to fetch user")]
	FetchUser,
	#[error("failed to fetch roles")]
	FetchRoles,
	#[error("failed to fetch session")]
	FetchSession,
	#[error("user not found")]
	UserNotFound,
}

impl From<AuthError> for RouteError<ApiError> {
	fn from(value: AuthError) -> Self {
		RouteError::from(match &value {
			AuthError::NotLoggedIn => (StatusCode::UNAUTHORIZED, "not logged in"),
			AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "invalid token"),
			AuthError::SessionExpired => (StatusCode::UNAUTHORIZED, "session expired"),
			AuthError::FetchGlobalState => (StatusCode::INTERNAL_SERVER_ERROR, "failed to fetch global state"),
			AuthError::FetchUser => (StatusCode::INTERNAL_SERVER_ERROR, "failed to fetch user"),
			AuthError::FetchRoles => (StatusCode::INTERNAL_SERVER_ERROR, "failed to fetch roles"),
			AuthError::FetchSession => (StatusCode::INTERNAL_SERVER_ERROR, "failed to fetch session"),
			AuthError::UserNotFound => (StatusCode::INTERNAL_SERVER_ERROR, "user not found"),
		})
		.with_source(Some(ApiError::Auth(value)))
	}
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct AuthData {
	pub session: Session,
	pub user_roles: Vec<Role>,
	pub user_permissions: RolePermission,
}

impl AuthData {
	pub async fn from_session_and_user<G: ApiGlobal>(
		global: &Arc<G>,
		session: Session,
		user: &User,
	) -> Result<Self, AuthError> {
		let Ok(Some(global_state)) = global.global_state_loader().load(()).await else {
			return Err(AuthError::FetchGlobalState);
		};

		let mut user_roles: Vec<Role> = global
			.role_by_id_loader()
			.load_many(user.roles.clone())
			.await
			.map_err(|_| AuthError::FetchRoles)?
			.into_values()
			.collect();

		// Computing user roles and permissions
		let global_roles_order = global_state
			.role_order
			.into_iter()
			.enumerate()
			.map(|(i, u)| (u, i))
			.collect::<HashMap<_, _>>();

		// Sort the user roles by index in global role order
		// Roles that are not included in the global role order list are considered
		// smallest
		user_roles.sort_by_key(|r| global_roles_order.get(&r.id));

		let user_permissions = user_roles
			.iter()
			.fold(global_state.default_permissions, |acc, role| acc.merge_with_role(role));

		Ok(Self {
			session,
			user_roles,
			user_permissions,
		})
	}

	pub async fn from_session<G: ApiGlobal>(global: &Arc<G>, session: Session) -> Result<Self, AuthError> {
		let user = global
			.user_by_id_loader()
			.load(session.user_id)
			.await
			.map_err(|_| AuthError::FetchUser)?
			.ok_or(AuthError::UserNotFound)?;

		Self::from_session_and_user(global, session, &user).await
	}

	pub async fn from_session_id<G: ApiGlobal>(global: &Arc<G>, session_id: Ulid) -> Result<Self, AuthError> {
		let session = global
			.session_by_id_loader()
			.load(session_id)
			.await
			.map_err(|_| AuthError::FetchSession)?
			.and_then(|s| s.is_valid().then_some(s))
			.ok_or(AuthError::SessionExpired)?;

		Self::from_session(global, session).await
	}
}
