use std::panic::Location;
use std::sync::Arc;

use async_graphql::ErrorExtensions;

use crate::api::auth::AuthError;
use crate::database::TotpError;
use crate::subscription::SubscriptionManagerError;
use crate::turnstile;

pub type Result<T, E = GqlErrorInterface> = std::result::Result<T, E>;

#[derive(Debug, Clone)]
pub struct GqlErrorInterface {
	error: GqlError,
	span: tracing::Span,
	location: &'static Location<'static>,
}

impl GqlErrorInterface {
	fn with_location(self, location: &'static Location<'static>) -> Self {
		Self { location, ..self }
	}

	fn with_source(self, source: Option<GqlError>) -> Self {
		Self {
			error: source.unwrap_or(self.error),
			..self
		}
	}
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum GqlError {
	/// An internal server error occurred.
	#[error("internal server error: {0}")]
	InternalServerError(&'static str),
	/// A database error occurred.
	#[error("database error: {0}")]
	Sqlx(#[from] Arc<sqlx::Error>),
	/// The input was invalid.
	#[error("invalid input for {fields:?}: {message}")]
	InvalidInput {
		fields: Vec<&'static str>,
		message: &'static str,
	},
	/// Auth error
	#[error("auth error: {0}")]
	Auth(#[from] AuthError),
	/// Not Implemented
	#[error("not implemented")]
	NotImplemented,
	/// Unauthorized
	#[error("unauthorized to see this field: {field}")]
	Unauthorized { field: &'static str },
	/// Not Found
	#[error("{0} was not found")]
	NotFound(&'static str),
	/// TOTP Error
	#[error("totp error: {0}")]
	Totp(#[from] TotpError),
	/// Turnstile Error
	#[error("turnstile error: {0}")]
	Turnstile(#[from] Arc<turnstile::TurnstileError>),
	/// Publish Error
	#[error("publish error: {0}")]
	Publish(#[from] Arc<async_nats::PublishError>),
	/// Subscription Error
	#[error("subscription error: {0}")]
	Subscription(#[from] Arc<SubscriptionManagerError>),
}

macro_rules! impl_arc_from {
	($err:ty) => {
		impl From<$err> for GqlError {
			fn from(err: $err) -> Self {
				Self::from(Arc::new(err))
			}
		}
	};
}

impl_arc_from!(sqlx::Error);
impl_arc_from!(turnstile::TurnstileError);
impl_arc_from!(async_nats::PublishError);
impl_arc_from!(SubscriptionManagerError);

impl GqlError {
	pub fn is_internal_server_error(&self) -> bool {
		matches!(
			self,
			GqlError::InternalServerError(_)
				| GqlError::Sqlx(_)
				| GqlError::Turnstile(_)
				| GqlError::Publish(_)
				| GqlError::Subscription(_)
		)
	}

	pub fn kind(&self) -> &'static str {
		if self.is_internal_server_error() {
			return "InternalServerError";
		}

		match self {
			GqlError::InvalidInput { .. } => "InvalidInput",
			GqlError::Auth(AuthError::NotLoggedIn) => "Auth(NotLoggedIn)",
			GqlError::Auth(AuthError::InvalidToken) => "Auth(InvalidToken)",
			GqlError::Auth(AuthError::SessionExpired) => "Auth(SessionExpired)",
			GqlError::Auth(AuthError::FetchGlobalState) => "Auth(FetchGlobalState)",
			GqlError::Auth(AuthError::FetchUser) => "Auth(FetchUser)",
			GqlError::Auth(AuthError::FetchRoles) => "Auth(FetchRoles)",
			GqlError::Auth(AuthError::FetchSession) => "Auth(FetchSession)",
			GqlError::Auth(AuthError::UserNotFound) => "Auth(UserNotFound)",
			GqlError::NotImplemented => "NotImplemented",
			GqlError::Unauthorized { .. } => "Unauthorized",
			GqlError::NotFound(_) => "NotFound",
			GqlError::Totp(_) => "Totp",
			GqlError::Turnstile(_) => "Turnstile",
			GqlError::Publish(_) => "Publish",
			GqlError::InternalServerError(_) => "InternalServerError",
			GqlError::Sqlx(_) => "Sqlx",
			GqlError::Subscription(_) => "Subscription",
		}
	}

	pub fn message(&self) -> String {
		match self {
			GqlError::InternalServerError(msg) => msg.to_string(),
			GqlError::InvalidInput { message, .. } => message.to_string(),
			_ => self.to_string(),
		}
	}

	pub fn fields(&self) -> Vec<&'static str> {
		match self {
			GqlError::InvalidInput { fields, .. } => fields.to_vec(),
			_ => Vec::new(),
		}
	}
}

impl ErrorExtensions for GqlErrorInterface {
	fn extend(&self) -> async_graphql::Error {
		let err = async_graphql::Error::new(self.error.to_string()).extend_with(|_, e| {
			e.set("kind", self.error.kind());
			e.set("reason", self.error.message());
			e.set("fields", self.error.fields());
		});

		self.span.in_scope(|| {
			if self.error.is_internal_server_error() {
				tracing::error!(
					error = %self.error,
					location = %self.location,
					"gql error",
				);
			} else {
				tracing::debug!(
					error = %self.error,
					location = %self.location,
					"gql error",
				);
			}
		});

		err
	}
}

impl<T> From<T> for GqlErrorInterface
where
	GqlError: From<T>,
{
	#[track_caller]
	fn from(value: T) -> Self {
		Self {
			error: GqlError::from(value),
			span: tracing::Span::current(),
			location: Location::caller(),
		}
	}
}

impl From<&'static str> for GqlErrorInterface {
	#[track_caller]
	fn from(msg: &'static str) -> Self {
		Self {
			error: GqlError::InternalServerError(msg),
			span: tracing::Span::current(),
			location: Location::caller(),
		}
	}
}

impl From<GqlErrorInterface> for async_graphql::Error {
	fn from(err: GqlErrorInterface) -> Self {
		err.extend()
	}
}

pub mod ext {
	use super::*;

	pub trait ResultExt<T, E>: Sized {
		fn map_err_gql<C>(self, ctx: C) -> Result<T>
		where
			GqlErrorInterface: From<C>,
			GqlError: From<E>;

		#[track_caller]
		fn map_err_ignored_gql<C>(self, ctx: C) -> Result<T, GqlErrorInterface>
		where
			GqlErrorInterface: From<C>;
	}

	impl<T, E> ResultExt<T, E> for std::result::Result<T, E> {
		#[track_caller]
		fn map_err_gql<C>(self, ctx: C) -> Result<T, GqlErrorInterface>
		where
			GqlErrorInterface: From<C>,
			GqlError: From<E>,
		{
			match self {
				Ok(v) => Ok(v),
				Err(err) => Err(GqlErrorInterface::from(ctx)
					.with_location(Location::caller())
					.with_source(Some(err.into()))),
			}
		}

		#[track_caller]
		fn map_err_ignored_gql<C>(self, ctx: C) -> Result<T, GqlErrorInterface>
		where
			GqlErrorInterface: From<C>,
		{
			match self {
				Ok(v) => Ok(v),
				Err(_) => Err(GqlErrorInterface::from(ctx).with_location(Location::caller())),
			}
		}
	}

	pub trait OptionExt<T>: Sized {
		fn map_err_gql<C>(self, ctx: C) -> Result<T>
		where
			GqlErrorInterface: From<C>;
	}

	impl<T> OptionExt<T> for std::option::Option<T> {
		#[track_caller]
		fn map_err_gql<C>(self, ctx: C) -> Result<T, GqlErrorInterface>
		where
			GqlErrorInterface: From<C>,
		{
			match self {
				Some(v) => Ok(v),
				None => Err(GqlErrorInterface::from(ctx).with_location(Location::caller())),
			}
		}
	}
}
