use std::{io, sync::Arc};

use serde_value::Value;

use crate::{KeyGraph, KeyPath};

pub type Result<T> = std::result::Result<T, ConfigError>;

/// Config error type
#[derive(Debug, thiserror::Error)]
pub enum ConfigErrorType {
    #[error("unsupported file format: {0}")]
    UnsupportedFileFormat(String),
    #[error("deserialize: {0}")]
    Deserialize(#[from] serde_value::DeserializerError),
    #[error("unsupported type: {0:?}")]
    UnsupportedType(Arc<KeyGraph>),
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("toml: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("yaml: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("multiple errors: {0}")]
    Multiple(MultiError),
    #[error("subkey on non-map: {0:?}")]
    SubkeyOnNonMap(Value),
    #[error("subkey on non-seq: {0:?}")]
    SubIndexOnNonSeq(Value),
    #[error("validation error: {0}")]
    ValidationError(String),
    #[error("invalid reference: {0}")]
    InvalidReference(&'static str),
    #[error("deserialized type not supported: {0}")]
    DeserializedTypeNotSupported(String),
    #[error("serialize: {0}")]
    Serialize(#[from] serde_value::SerializerError),
}

#[derive(Debug)]
pub struct MultiError(Vec<ConfigError>);

impl MultiError {
    pub fn into_inner(self) -> Vec<ConfigError> {
        self.0
    }

    pub fn inner(&self) -> &[ConfigError] {
        &self.0
    }
}

impl std::fmt::Display for MultiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let errors = self
            .0
            .iter()
            .map(|e| format!("{}", e))
            .collect::<Vec<_>>()
            .join(", ");

        write!(f, "{}", errors)
    }
}

/// The source of an error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorSource {
    Cli,
    Env,
    File(String),
    Manual,
}

/// General config error
#[derive(Debug, Clone)]
pub struct ConfigError {
    error: Arc<ConfigErrorType>,
    path: Option<KeyPath>,
    source: Option<ErrorSource>,
}

impl ConfigError {
    pub(crate) fn new(error: ConfigErrorType) -> Self {
        Self {
            error: Arc::new(error),
            path: None,
            source: None,
        }
    }

    pub(crate) fn with_path(mut self, path: KeyPath) -> Self {
        self.path = Some(path);
        self
    }

    pub(crate) fn with_source(mut self, source: ErrorSource) -> Self {
        self.source = Some(source);
        self
    }

    /// Returns true if the error is an IO error.
    pub fn is_io(&self) -> bool {
        match self.error() {
            ConfigErrorType::Io(_) => true,
            ConfigErrorType::Multiple(MultiError(errors)) => errors.iter().any(|e| e.is_io()),
            _ => false,
        }
    }

    /// Returns the path of the error.
    pub fn path(&self) -> Option<&KeyPath> {
        self.path.as_ref()
    }

    /// Returns the source of the error.
    pub fn source(&self) -> Option<&ErrorSource> {
        self.source.as_ref()
    }

    /// Returns the error type.
    pub fn error(&self) -> &ConfigErrorType {
        &self.error
    }

    /// Creates a new multi error from this error and another.
    pub fn multi(self, other: Self) -> Self {
        let mut errors = Vec::new();
        match self.error() {
            ConfigErrorType::Multiple(MultiError(errors1)) => {
                errors.extend(errors1.clone());
            }
            _ => {
                errors.push(self);
            }
        }

        match other.error() {
            ConfigErrorType::Multiple(MultiError(errors2)) => {
                errors.extend(errors2.clone());
            }
            _ => {
                errors.push(other);
            }
        }

        Self::new(ConfigErrorType::Multiple(MultiError(errors)))
    }
}

impl From<ConfigErrorType> for ConfigError {
    fn from(value: ConfigErrorType) -> Self {
        Self::new(value)
    }
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Some errors will have a path and or source, so we should include those in the output.
        // We should also write better errors for the enum variants.
        if let Some(source) = self.source() {
            write!(f, "{:?}: ", source)?;
        }

        self.error().fmt(f)?;

        if let Some(path) = &self.path {
            write!(f, " (path .{})", path)?;
        }

        Ok(())
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.error)
    }
}
