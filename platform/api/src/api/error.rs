use std::{
    fmt::{Debug, Display},
    panic::Location,
};

use hyper::{Body, StatusCode};
use serde_json::json;

use super::{macros::make_response, middleware::auth::AuthError};

pub type Result<T, E = ApiErrorInterface> = std::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("failed to upgrade ws connection: {0}")]
    WsUpgrade(hyper_tungstenite::tungstenite::error::ProtocolError),
    #[error("no gql request found")]
    NoGqlRequest,
    #[error("failed to parse http body: {0}")]
    ParseHttpBody(hyper::Error),
    #[error("invalid ws protocol")]
    InvalidWsProtocol,
    #[error("failed to parse gql request: {0}")]
    ParseGql(#[from] async_graphql::ParseRequestError),
    #[error("http method not allowed")]
    HttpMethodNotAllowed,
    #[error("failed to auth: {0}")]
    Auth(#[from] AuthError),
    #[error("internal server error: {0}")]
    InternalServerError(&'static str),
}

impl From<ApiError> for hyper::Response<Body> {
    fn from(value: ApiError) -> Self {
        let status = match &value {
            ApiError::WsUpgrade(_) => StatusCode::BAD_REQUEST,
            ApiError::NoGqlRequest => StatusCode::BAD_REQUEST,
            ApiError::ParseHttpBody(_) => StatusCode::BAD_REQUEST,
            ApiError::InvalidWsProtocol => StatusCode::BAD_REQUEST,
            ApiError::ParseGql(_) => StatusCode::BAD_REQUEST,
            ApiError::HttpMethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
            ApiError::Auth(_) => StatusCode::UNAUTHORIZED,
            ApiError::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        make_response!(
            status,
            json!({ "message": value.to_string(), "success": false })
        )
    }
}

pub struct ApiErrorInterface {
    error: ApiError,
    location: &'static Location<'static>,
    span: tracing::Span,
}

impl ApiErrorInterface {
    pub fn span(&self) -> &tracing::Span {
        &self.span
    }

    pub fn location(&self) -> &'static Location<'static> {
        self.location
    }

    pub fn error(&self) -> &ApiError {
        &self.error
    }

    pub fn response(self) -> hyper::Response<Body> {
        self.error.into()
    }

    fn with_location(self, location: &'static Location<'static>) -> Self {
        Self { location, ..self }
    }
}

impl From<&'static str> for ApiErrorInterface {
    #[track_caller]
    fn from(message: &'static str) -> Self {
        Self {
            error: ApiError::InternalServerError(message),
            span: tracing::Span::current(),
            location: Location::caller(),
        }
    }
}

impl<T> From<T> for ApiErrorInterface
where
    ApiError: From<T>,
{
    #[track_caller]
    fn from(error: T) -> Self {
        Self {
            error: ApiError::from(error),
            span: tracing::Span::current(),
            location: Location::caller(),
        }
    }
}

impl Debug for ApiErrorInterface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.error, f)
    }
}

impl Display for ApiErrorInterface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.error, f)
    }
}

impl std::error::Error for ApiErrorInterface {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

pub trait ResultExt<T, E>: Sized {
    fn map_err_route<C>(self, ctx: C) -> Result<T>
    where
        ApiErrorInterface: From<C>;
}

impl<T> ResultExt<T, ()> for std::option::Option<T> {
    #[track_caller]
    fn map_err_route<C>(self, ctx: C) -> Result<T>
    where
        ApiErrorInterface: From<C>,
    {
        match self {
            Some(val) => Ok(val),
            None => Err(ApiErrorInterface::from(ctx).with_location(Location::caller())),
        }
    }
}

impl<T, E> ResultExt<T, E> for std::result::Result<T, E>
where
    ApiError: From<E>,
{
    #[track_caller]
    fn map_err_route<C>(self, ctx: C) -> Result<T>
    where
        ApiErrorInterface: From<C>,
    {
        match self {
            Ok(val) => Ok(val),
            Err(_) => Err(ApiErrorInterface::from(ctx).with_location(Location::caller())),
        }
    }
}
