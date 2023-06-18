//! File source
//!
//! ```
//! # use config::sources;
//! #
//! # #[derive(config::Config, serde::Deserialize)]
//! # struct MyConfig {
//! #     // ...
//! # }
//! #
//! # fn main() -> Result<(), config::ConfigError> {
//! let mut builder = config::ConfigBuilder::new();
//! // Add FileSource with path config.toml
//! builder.add_source(sources::FileSource::with_path("config.toml")?);
//! // Build the final configuration
//! let config: MyConfig = builder.build()?;
//! # Ok(())
//! # }
//! ```

use std::{
    fs,
    io::{self, Read},
    marker::PhantomData,
    path::Path,
};

use crate::{Config, ConfigError, ConfigErrorType, ErrorSource, KeyPath, Result, Source, Value};

use super::utils;

mod json;
mod toml;
mod yaml;

/// File source
///
/// Create a new file source with [`FileSource::with_path`](FileSource::with_path).
///
/// When you have a reader (implementing [`Read`](Read)), you can use [`FileSource::json`](FileSource::json), [`FileSource::toml`](FileSource::toml) or [`FileSource::yaml`](FileSource::yaml) depending on the file format.
pub struct FileSource<C: Config> {
    content: Value,
    location: String,
    _phantom: PhantomData<C>,
}

impl<C: Config> FileSource<C> {
    /// Creates a new file source with a given path.
    ///
    /// The file format is determined by the extension of the path.
    /// It can be TOML, YAML or JSON.
    /// When the extension of the file is not supported, the function will try to parse the file as all supported formats and return the first one that succeeds.
    pub fn with_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let fs_fn = match path.as_ref().extension().and_then(|s| s.to_str()) {
            Some("json") => Self::json,
            Some("toml") => Self::toml,
            Some("yaml") | Some("yml") => Self::yaml,
            None => {
                // We should try to parse the file as all supported formats
                // and return the first one that succeeds
                let fn_mut = |e: Option<ConfigError>, ext: &str| {
                    if e.as_ref().map(|e| e.is_io()).unwrap_or(true) {
                        Self::with_path(path.as_ref().with_extension(ext))
                    } else {
                        Err(e.unwrap())
                    }
                };

                return fn_mut(None, "json")
                    .or_else(|err| fn_mut(Some(err), "toml"))
                    .or_else(|err| fn_mut(Some(err), "yaml"))
                    .or_else(|err| fn_mut(Some(err), "yml"));
            }
            ext => {
                return Err(ConfigErrorType::UnsupportedFileFormat(
                    ext.map(|s| s.to_string()).unwrap_or_default(),
                )
                .into())
            }
        };

        let mut fs = match fs::File::open(&path).map(fs_fn) {
            Ok(Ok(fs)) => Ok(fs),
            Ok(Err(err)) => Err(err),
            Err(err) => Err(ConfigErrorType::Io(err).into()),
        }
        .map_err(|e| e.with_source(ErrorSource::File(path.as_ref().display().to_string())))?;

        fs.location = path.as_ref().display().to_string();

        Ok(fs)
    }

    pub fn location(&self) -> &str {
        self.location.as_str()
    }

    /// Creates a new file source from a given reader in JSON format.
    pub fn json<R: Read>(reader: R) -> Result<Self> {
        let content: serde_json::Value = serde_json::from_reader(reader).map_err(|e| {
            ConfigError::new(ConfigErrorType::Json(e))
                .with_source(ErrorSource::File("json bytes".to_string()))
        })?;
        Ok(Self {
            content: json::convert_value(&KeyPath::root(), content)
                .and_then(|value| C::transform(&KeyPath::root(), value))
                .map_err(|e| e.with_source(ErrorSource::File("json bytes".to_string())))?,
            _phantom: PhantomData,
            location: "json bytes".to_string(),
        })
    }

    /// Creates a new file source from a given reader in TOML format.
    pub fn toml<R: Read>(reader: R) -> Result<Self> {
        let content = io::read_to_string(reader).map_err(|e| {
            ConfigError::new(ConfigErrorType::Io(e))
                .with_source(ErrorSource::File("toml bytes".to_string()))
        })?;
        let value: ::toml::Value = ::toml::from_str(&content).map_err(|e| {
            ConfigError::new(ConfigErrorType::Toml(e))
                .with_source(ErrorSource::File("toml bytes".to_string()))
        })?;
        Ok(Self {
            content: toml::convert_value(&KeyPath::root(), value)
                .and_then(|value| C::transform(&KeyPath::root(), value))
                .map_err(|e| e.with_source(ErrorSource::File("toml bytes".to_string())))?,
            _phantom: PhantomData,
            location: "toml bytes".to_string(),
        })
    }

    /// Creates a new file source from a given reader in YAML format.
    pub fn yaml<R: Read>(reader: R) -> Result<Self> {
        let content: serde_yaml::Value = serde_yaml::from_reader(reader).map_err(|e| {
            ConfigError::new(ConfigErrorType::Yaml(e))
                .with_source(ErrorSource::File("yaml bytes".to_string()))
        })?;
        Ok(Self {
            content: yaml::convert_value(&KeyPath::root(), content)
                .and_then(|value| C::transform(&KeyPath::root(), value))
                .map_err(|e| e.with_source(ErrorSource::File("yaml bytes".to_string())))?,
            _phantom: PhantomData,
            location: "yaml bytes".to_string(),
        })
    }
}

impl<C: Config> Source<C> for FileSource<C> {
    fn get_key(&self, path: &KeyPath) -> Result<Option<Value>> {
        utils::get_key::<C>(&self.content, path)
            .map_err(|e| e.with_source(ErrorSource::File(self.location.clone())))
    }
}
