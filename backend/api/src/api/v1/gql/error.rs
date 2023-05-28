use std::{
    fmt::{Display, Formatter},
    panic::Location,
    sync::Arc,
};

use async_graphql::ErrorExtensions;

pub type Result<T, E = GqlErrorInterface> = std::result::Result<T, E>;

#[derive(Clone)]
pub struct GqlErrorInterface {
    kind: GqlError,
    message: Option<String>,
    fields: Vec<String>,
    span: tracing::Span,
    source: Option<Arc<anyhow::Error>>,
    location: &'static Location<'static>,
}

impl GqlErrorInterface {
    fn with_source(self, source: Option<anyhow::Error>) -> Self {
        Self {
            source: source.map(Arc::new),
            ..self
        }
    }

    fn with_location(self, location: &'static Location<'static>) -> Self {
        Self { location, ..self }
    }

    pub fn with_field(self, fields: Vec<&str>) -> Self {
        let fields = fields.into_iter().map(|f| f.to_string()).collect();
        Self { fields, ..self }
    }

    pub fn with_message(self, message: String) -> Self {
        Self {
            message: Some(message),
            ..self
        }
    }

    fn display(&self) -> String {
        match &self.message {
            Some(msg) => format!("{}: {}", self.kind, msg),
            None => format!("{}", self.kind),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]

pub enum GqlError {
    /// An internal server error occurred.
    InternalServerError,
    /// The input was invalid.
    InvalidInput,
    /// The session is no longer valid.
    InvalidSession,
    /// Not Implemented
    NotImplemented,
    /// Unauthorized
    Unauthorized,
    /// Not Found
    NotFound,
}

impl Display for GqlError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GqlError::InternalServerError => write!(f, "InternalServerError"),
            GqlError::InvalidInput => write!(f, "InvalidInput"),
            GqlError::InvalidSession => write!(f, "InvalidSession"),
            GqlError::NotImplemented => write!(f, "NotImplemented"),
            GqlError::Unauthorized => write!(f, "Unauthorized"),
            GqlError::NotFound => write!(f, "NotFound"),
        }
    }
}

impl GqlError {
    #[track_caller]
    pub fn with_message(self, message: &str) -> GqlErrorInterface {
        GqlErrorInterface {
            kind: self,
            message: Some(message.to_string()),
            span: tracing::Span::current(),
            source: None,
            fields: Vec::new(),
            location: Location::caller(),
        }
    }
}

impl ErrorExtensions for GqlErrorInterface {
    fn extend(&self) -> async_graphql::Error {
        let err = async_graphql::Error::new(self.display()).extend_with(|_, e| {
            e.set("kind", self.kind.to_string());
            if let Some(message) = &self.message {
                e.set("reason", message.as_str());
            }

            e.set("fields", self.fields.as_slice());
        });

        match self.kind {
            GqlError::InternalServerError => {
                self.span.in_scope(|| {
                    tracing::error!(error = ?self.source, location = self.location.to_string(), "gql error: {}", self.display());
                });
            }
            _ => {
                self.span.in_scope(|| {
                    tracing::debug!(error = ?self.source, location = self.location.to_string(), "gql error: {}", self.display());
                });
            }
        }

        err
    }
}

impl From<(GqlError, &'_ str)> for GqlErrorInterface {
    #[track_caller]
    fn from((kind, message): (GqlError, &'_ str)) -> Self {
        Self {
            kind,
            message: Some(message.to_string()),
            span: tracing::Span::current(),
            source: None,
            fields: Vec::new(),
            location: Location::caller(),
        }
    }
}

impl From<GqlError> for GqlErrorInterface {
    #[track_caller]
    fn from(kind: GqlError) -> Self {
        Self {
            kind,
            message: None,
            span: tracing::Span::current(),
            source: None,
            fields: Vec::new(),
            location: Location::caller(),
        }
    }
}

impl
    From<(
        GqlError,
        &'_ str,
        &'static (dyn std::error::Error + Sync + Send),
    )> for GqlErrorInterface
{
    #[track_caller]
    fn from(
        (kind, message, err): (
            GqlError,
            &'_ str,
            &'static (dyn std::error::Error + Sync + Send),
        ),
    ) -> Self {
        Self {
            kind,
            fields: Vec::new(),
            message: Some(message.to_string()),
            span: tracing::Span::current(),
            source: Some(Arc::new(err.into())),
            location: Location::caller(),
        }
    }
}

impl From<&'_ str> for GqlErrorInterface {
    #[track_caller]
    fn from(msg: &'_ str) -> Self {
        Self {
            fields: Vec::new(),
            kind: GqlError::InternalServerError,
            message: Some(msg.to_string()),
            span: tracing::Span::current(),
            source: None,
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

impl<T, E> ResultExt<T, E> for std::result::Result<T, E>
where
    anyhow::Error: From<E>,
{
    #[track_caller]
    fn map_err_gql<C>(self, ctx: C) -> Result<T, GqlErrorInterface>
    where
        GqlErrorInterface: From<C>,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(GqlErrorInterface::from(ctx)
                .with_source(Some(e.into()))
                .with_location(Location::caller())),
        }
    }
}
