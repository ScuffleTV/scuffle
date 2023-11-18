use std::fmt::{Debug, Display};
use std::panic::Location;

use http::Response;
use hyper::{Body, StatusCode};
use routerify::RequestInfo;
use serde_json::json;

#[macro_export]
macro_rules! make_response {
	($status:expr, $body:expr) => {
		hyper::Response::builder()
			.status($status)
			.header("Content-Type", "application/json")
			.body(Body::from($body.to_string()))
			.expect("failed to build response")
	};
}

pub async fn error_handler<E: std::error::Error + 'static>(
	err: Box<(dyn std::error::Error + Send + Sync + 'static)>,
	info: RequestInfo,
) -> Response<Body> {
	match err.downcast::<RouteError<E>>() {
		Ok(err) => {
			let location = err.location();

			err.span().in_scope(|| match err.should_log() {
				ShouldLog::Yes => {
					tracing::error!(path = %info.uri(), method = %info.method(), location = location.to_string(), error = ?err, "http error")
				}
				ShouldLog::Debug => {
					tracing::debug!(path = %info.uri(), method = %info.method(), location = location.to_string(), error = ?err, "http error")
				}
				ShouldLog::No => (),
			});

			err.response()
		}
		Err(err) => {
			tracing::error!(path = %info.uri(), method = %info.method(), error = ?err, info = ?info, "unhandled http error");
			make_response!(
				StatusCode::INTERNAL_SERVER_ERROR,
				json!({ "message": "Internal Server Error", "success": false })
			)
		}
	}
}

pub struct RouteError<E> {
	source: Option<E>,
	location: &'static Location<'static>,
	span: tracing::Span,
	response: hyper::Response<Body>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShouldLog {
	Yes,
	Debug,
	No,
}

impl<E> RouteError<E> {
	pub fn span(&self) -> &tracing::Span {
		&self.span
	}

	pub fn location(&self) -> &'static Location<'static> {
		self.location
	}

	pub fn response(self) -> hyper::Response<Body> {
		self.response
	}

	pub fn should_log(&self) -> ShouldLog {
		match self.response.status().is_server_error() {
			true => ShouldLog::Yes,
			false => match self.source.is_some() {
				true => ShouldLog::Debug,
				false => ShouldLog::No,
			},
		}
	}

	pub fn with_source(mut self, source: Option<E>) -> Self {
		self.source = source;
		self
	}

	pub fn with_location(mut self, location: &'static Location<'static>) -> Self {
		self.location = location;
		self
	}
}

impl<E> From<hyper::Response<Body>> for RouteError<E> {
	#[track_caller]
	fn from(res: hyper::Response<Body>) -> Self {
		Self {
			source: None,
			span: tracing::Span::current(),
			location: Location::caller(),
			response: res,
		}
	}
}

impl<E, S: AsRef<str>> From<(StatusCode, S)> for RouteError<E> {
	#[track_caller]
	fn from(status: (StatusCode, S)) -> Self {
		Self {
			source: None,
			span: tracing::Span::current(),
			location: Location::caller(),
			response: make_response!(status.0, json!({ "message": status.1.as_ref(), "success": false })),
		}
	}
}

impl<E, S: AsRef<str>, T> From<(StatusCode, S, T)> for RouteError<E>
where
	T: Into<E>,
{
	#[track_caller]
	fn from(status: (StatusCode, S, T)) -> Self {
		Self {
			source: Some(status.2.into()),
			span: tracing::Span::current(),
			location: Location::caller(),
			response: make_response!(status.0, json!({ "message": status.1.as_ref(), "success": false })),
		}
	}
}

impl<E> From<&'_ str> for RouteError<E> {
	#[track_caller]
	fn from(message: &'_ str) -> Self {
		Self {
			source: None,
			span: tracing::Span::current(),
			location: Location::caller(),
			response: make_response!(
				StatusCode::INTERNAL_SERVER_ERROR,
				json!({ "message": message, "success": false })
			),
		}
	}
}

impl<E: Debug> Debug for RouteError<E> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match &self.source {
			Some(err) => write!(f, "RouteError: {:?}", err),
			None => write!(f, "RouteError: Unknown Source"),
		}
	}
}

impl<E: Display> Display for RouteError<E> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match &self.source {
			Some(err) => write!(f, "RouteError: {}", err),
			None => write!(f, "RouteError: Unknown Source"),
		}
	}
}

impl<E: std::error::Error + 'static> std::error::Error for RouteError<E> {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match &self.source {
			Some(err) => Some(err),
			None => None,
		}
	}
}

pub mod ext {
	use std::panic::Location;
	use std::sync::{Arc, Weak};

	use http::StatusCode;

	use super::RouteError;

	pub trait ResultExt<T, E, E2>: Sized {
		fn map_err_route<C>(self, ctx: C) -> std::result::Result<T, RouteError<E2>>
		where
			RouteError<E2>: From<C>,
			E2: From<E>;

		fn map_ignore_err_route<C>(self, ctx: C) -> std::result::Result<T, RouteError<E2>>
		where
			RouteError<E2>: From<C>;

		fn into_err_route(self) -> std::result::Result<T, RouteError<E2>>
		where
			RouteError<E2>: From<E>;
	}

	impl<T, E, E2> ResultExt<T, E, E2> for std::result::Result<T, E> {
		#[track_caller]
		fn map_err_route<C>(self, ctx: C) -> std::result::Result<T, RouteError<E2>>
		where
			RouteError<E2>: From<C>,
			E2: From<E>,
		{
			match self {
				Ok(val) => Ok(val),
				Err(err) => Err(RouteError::from(ctx)
					.with_source(Some(err.into()))
					.with_location(Location::caller())),
			}
		}

		#[track_caller]
		fn map_ignore_err_route<C>(self, ctx: C) -> std::result::Result<T, RouteError<E2>>
		where
			RouteError<E2>: From<C>,
		{
			match self {
				Ok(val) => Ok(val),
				Err(_) => Err(RouteError::from(ctx).with_location(Location::caller())),
			}
		}

		#[track_caller]
		fn into_err_route(self) -> std::result::Result<T, RouteError<E2>>
		where
			RouteError<E2>: From<E>,
		{
			match self {
				Ok(val) => Ok(val),
				Err(err) => Err(RouteError::from(err).with_location(Location::caller())),
			}
		}
	}

	pub trait OptionExt<T, E>: Sized {
		fn map_err_route<C>(self, ctx: C) -> std::result::Result<T, RouteError<E>>
		where
			RouteError<E>: From<C>;
	}

	impl<T, E> OptionExt<T, E> for std::option::Option<T> {
		#[track_caller]
		fn map_err_route<C>(self, ctx: C) -> std::result::Result<T, RouteError<E>>
		where
			RouteError<E>: From<C>,
		{
			match self {
				Some(val) => Ok(val),
				None => Err(RouteError::from(ctx).with_location(Location::caller())),
			}
		}
	}

	pub trait RequestGlobalExt<E> {
		fn get_global<G: Sync + Send + 'static>(&self) -> std::result::Result<Arc<G>, RouteError<E>>;
	}

	impl<E, B> RequestGlobalExt<E> for hyper::Request<B>
	where
		Self: routerify::ext::RequestExt,
	{
		fn get_global<G: Sync + Send + 'static>(&self) -> std::result::Result<Arc<G>, RouteError<E>> {
			use routerify::ext::RequestExt;

			Ok(self
				.data::<Weak<G>>()
				.expect("global state not set")
				.upgrade()
				.ok_or((StatusCode::INTERNAL_SERVER_ERROR, "failed to upgrade global state"))?)
		}
	}
}
