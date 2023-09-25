use async_graphql::ErrorExtensions;
use std::panic::Location;

pub type Result<T, E = GqlErrorInterface> = std::result::Result<T, E>;

#[derive(Clone)]
pub struct GqlErrorInterface {
    error: GqlError,
    span: tracing::Span,
    location: &'static Location<'static>,
}

impl GqlErrorInterface {
    fn with_location(self, location: &'static Location<'static>) -> Self {
        Self { location, ..self }
    }
}

#[derive(PartialEq, Eq, Clone, Debug, thiserror::Error)]
pub enum GqlError {
    /// An internal server error occurred.
    #[error("internal server error: {0}")]
    InternalServerError(&'static str),
    /// The input was invalid.
    #[error("invalid input for {fields:?}: {message}")]
    InvalidInput {
        fields: Vec<&'static str>,
        message: &'static str,
    },
    /// The session is no longer valid.
    #[error("invalid session")]
    InvalidSession,
    /// Not Implemented
    #[error("not implemented")]
    NotImplemented,
    /// Unauthorized
    #[error("unauthorized to see this field: {field}")]
    Unauthorized { field: &'static str },
    /// Not Logged In
    #[error("not logged in")]
    NotLoggedIn,
    /// Not Found
    #[error("{0} not found")]
    NotFound(&'static str),
}

impl GqlError {
    pub fn kind(&self) -> &'static str {
        match self {
            GqlError::InternalServerError(_) => "InternalServerError",
            GqlError::InvalidInput { .. } => "InvalidInput",
            GqlError::InvalidSession => "InvalidSession",
            GqlError::NotImplemented => "NotImplemented",
            GqlError::Unauthorized { .. } => "Unauthorized",
            GqlError::NotLoggedIn => "NotLoggedIn",
            GqlError::NotFound(_) => "NotFound",
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

        match self.error {
            GqlError::InternalServerError(_) => {
                self.span.in_scope(|| {
                    tracing::error!(
                        error = self.error.to_string(),
                        location = self.location.to_string(),
                        "gql error: {}",
                        self.error
                    );
                });
            }
            _ => {
                self.span.in_scope(|| {
                    tracing::debug!(
                        error = self.error.to_string(),
                        location = self.location.to_string(),
                        "gql error: {}",
                        self.error
                    );
                });
            }
        }

        err
    }
}

impl From<GqlError> for GqlErrorInterface {
    #[track_caller]
    fn from(error: GqlError) -> Self {
        Self {
            error,
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

pub trait ResultExt<T, E>: Sized {
    fn map_err_gql<C>(self, ctx: C) -> Result<T>
    where
        GqlErrorInterface: From<C>;
}

impl<T, E> ResultExt<T, E> for std::result::Result<T, E> {
    #[track_caller]
    fn map_err_gql<C>(self, ctx: C) -> Result<T, GqlErrorInterface>
    where
        GqlErrorInterface: From<C>,
    {
        match self {
            Ok(v) => Ok(v),
            Err(_) => Err(GqlErrorInterface::from(ctx).with_location(Location::caller())),
        }
    }
}

impl<T> ResultExt<T, ()> for std::option::Option<T> {
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
