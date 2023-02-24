use std::{
    fmt::{Debug, Display},
    panic::Location,
};

use hyper::{Body, StatusCode};
use serde_json::json;

use super::macros::make_response;

pub type Result<T, E = RouteError> = std::result::Result<T, E>;

pub struct RouteError {
    source: Option<anyhow::Error>,
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

impl RouteError {
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

    fn with_source(self, source: Option<anyhow::Error>) -> Self {
        Self { source, ..self }
    }

    fn with_location(self, location: &'static Location<'static>) -> Self {
        Self { location, ..self }
    }
}

impl From<hyper::Response<Body>> for RouteError {
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

impl From<(StatusCode, &'_ str)> for RouteError {
    #[track_caller]
    fn from(status: (StatusCode, &'_ str)) -> Self {
        Self {
            source: None,
            span: tracing::Span::current(),
            location: Location::caller(),
            response: make_response!(status.0, json!({ "message": status.1, "success": false })),
        }
    }
}

impl<T> From<(StatusCode, &'_ str, T)> for RouteError
where
    T: Into<anyhow::Error> + Debug + Display,
{
    #[track_caller]
    fn from(status: (StatusCode, &'_ str, T)) -> Self {
        Self {
            source: Some(status.2.into()),
            span: tracing::Span::current(),
            location: Location::caller(),
            response: make_response!(status.0, json!({ "message": status.1, "success": false })),
        }
    }
}

impl From<&'_ str> for RouteError {
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

impl Debug for RouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.source {
            Some(err) => write!(f, "RouteError: {:?}", err),
            None => write!(f, "RouteError: Unknown Source"),
        }
    }
}

impl Display for RouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.source {
            Some(err) => write!(f, "RouteError: {}", err),
            None => write!(f, "RouteError: Unknown Source"),
        }
    }
}

impl std::error::Error for RouteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.source {
            Some(err) => Some(err.as_ref()),
            None => None,
        }
    }
}

pub trait ResultExt<T, E>: Sized {
    fn extend_route<C>(self, ctx: C) -> Result<T>
    where
        RouteError: From<C>;
}

impl<T, E> ResultExt<T, E> for std::result::Result<T, E>
where
    anyhow::Error: From<E>,
{
    #[track_caller]
    fn extend_route<C>(self, ctx: C) -> Result<T>
    where
        RouteError: From<C>,
    {
        match self {
            Ok(val) => Ok(val),
            Err(err) => Err(RouteError::from(ctx)
                .with_source(Some(err.into()))
                .with_location(Location::caller())),
        }
    }
}
