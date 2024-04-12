use std::fmt::{Debug, Display};
use std::panic::Location;

use bytes::Bytes;
use http::StatusCode;
use http_body_util::Full;
use serde_json::json;

pub mod router;

#[macro_export]
macro_rules! make_response {
	($status:expr, $body:expr) => {
		hyper::Response::builder()
			.status($status)
			.header("Content-Type", "application/json")
			.body(::hyper::body::Bytes::from($body.to_string()).into())
			.expect("failed to build response")
	};
}

pub async fn error_handler<E: std::error::Error + 'static, B: From<Bytes>>(
	req: hyper::Request<()>,
	err: RouteError<E, B>,
) -> hyper::Response<B> {
	let location = err.location();

	err.span().in_scope(|| match err.should_log() {
		ShouldLog::Yes => {
			tracing::error!(path = %req.uri(), method = %req.method(), location = location.to_string(), error = ?err, "http error")
		}
		ShouldLog::Debug => {
			tracing::debug!(path = %req.uri(), method = %req.method(), location = location.to_string(), error = ?err, "http error")
		}
		ShouldLog::No => (),
	});

	err.response()
}

pub struct RouteError<E, B: From<Bytes> = Full<Bytes>> {
	source: Option<E>,
	location: &'static Location<'static>,
	span: tracing::Span,
	response: hyper::Response<B>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShouldLog {
	Yes,
	Debug,
	No,
}

impl<E, B: From<Bytes>> RouteError<E, B> {
	pub fn span(&self) -> &tracing::Span {
		&self.span
	}

	pub fn location(&self) -> &'static Location<'static> {
		self.location
	}

	pub fn response(self) -> hyper::Response<B> {
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

impl<E, B: From<Bytes>> From<hyper::Response<Bytes>> for RouteError<E, B> {
	#[track_caller]
	fn from(res: hyper::Response<Bytes>) -> Self {
		Self {
			source: None,
			span: tracing::Span::current(),
			location: Location::caller(),
			response: res.map(|b| b.into()),
		}
	}
}

impl<E, S: AsRef<str>, B: From<Bytes>> From<(StatusCode, S)> for RouteError<E, B> {
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

impl<E, S: AsRef<str>, T, B: From<Bytes>> From<(StatusCode, S, T)> for RouteError<E, B>
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

impl<E, B: From<Bytes>> From<&'_ str> for RouteError<E, B> {
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

impl<E: Debug, B: From<Bytes>> Debug for RouteError<E, B> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match &self.source {
			Some(err) => write!(f, "RouteError: {:?}", err),
			None => write!(f, "RouteError: Unknown Source"),
		}
	}
}

impl<E: Display, B: From<Bytes>> Display for RouteError<E, B> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match &self.source {
			Some(err) => write!(f, "RouteError: {}", err),
			None => write!(f, "RouteError: Unknown Source"),
		}
	}
}

impl<E: std::error::Error + 'static, B: From<Bytes>> std::error::Error for RouteError<E, B> {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match &self.source {
			Some(err) => Some(err),
			None => None,
		}
	}
}

pub mod ext {
	use std::panic::Location;

	use bytes::Bytes;

	use super::RouteError;

	pub trait ResultExt<T, E, E2>: Sized {
		fn map_err_route<C, B: From<Bytes>>(self, ctx: C) -> std::result::Result<T, RouteError<E2, B>>
		where
			RouteError<E2, B>: From<C>,
			E2: From<E>;

		fn map_ignore_err_route<C, B: From<Bytes>>(self, ctx: C) -> std::result::Result<T, RouteError<E2, B>>
		where
			RouteError<E2, B>: From<C>;

		fn into_err_route<B: From<Bytes>>(self) -> std::result::Result<T, RouteError<E2, B>>
		where
			RouteError<E2, B>: From<E>;
	}

	impl<T, E, E2> ResultExt<T, E, E2> for std::result::Result<T, E> {
		#[track_caller]
		fn map_err_route<C, B: From<Bytes>>(self, ctx: C) -> std::result::Result<T, RouteError<E2, B>>
		where
			RouteError<E2, B>: From<C>,
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
		fn map_ignore_err_route<C, B: From<Bytes>>(self, ctx: C) -> std::result::Result<T, RouteError<E2, B>>
		where
			RouteError<E2, B>: From<C>,
		{
			match self {
				Ok(val) => Ok(val),
				Err(_) => Err(RouteError::from(ctx).with_location(Location::caller())),
			}
		}

		#[track_caller]
		fn into_err_route<B: From<Bytes>>(self) -> std::result::Result<T, RouteError<E2, B>>
		where
			RouteError<E2, B>: From<E>,
		{
			match self {
				Ok(val) => Ok(val),
				Err(err) => Err(RouteError::from(err).with_location(Location::caller())),
			}
		}
	}

	pub trait OptionExt<T, E>: Sized {
		fn map_err_route<C, B: From<Bytes>>(self, ctx: C) -> std::result::Result<T, RouteError<E, B>>
		where
			RouteError<E, B>: From<C>;
	}

	impl<T, E> OptionExt<T, E> for std::option::Option<T> {
		#[track_caller]
		fn map_err_route<C, B: From<Bytes>>(self, ctx: C) -> std::result::Result<T, RouteError<E, B>>
		where
			RouteError<E, B>: From<C>,
		{
			match self {
				Some(val) => Ok(val),
				None => Err(RouteError::from(ctx).with_location(Location::caller())),
			}
		}
	}
}
