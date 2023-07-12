//! Manual source
//!
//! A manual source lets you set values manually.
//!
//! ```
//! # use config::{sources, Value};
//! #
//! # #[derive(config::Config, serde::Deserialize)]
//! # struct MyConfig {
//! #     // ...
//! # }
//! #
//! # fn main() -> Result<(), config::ConfigError> {
//! let mut builder = config::ConfigBuilder::new();
//! // Create a new ManualSource
//! let mut manual = sources::ManualSource::new();
//! manual.set("test.foo", Value::Bool(true));
//! // Add ManualSource
//! builder.add_source(manual);
//! // Build the final configuration
//! let config: MyConfig = builder.build()?;
//! # Ok(())
//! # }
//! ```

use std::{collections::BTreeMap, marker::PhantomData};

use crate::{
    Config, ConfigError, ConfigErrorType, ErrorSource, KeyPath, KeyPathSegment, Result, Source,
    Value,
};

use super::utils;

/// Manual source
///
/// Create a new manual source with [`ManualSource::new`](ManualSource::new).
pub struct ManualSource<C: Config> {
    value: Option<Value>,
    _phantom: PhantomData<C>,
}

fn value_to_value_graph(path: KeyPath, mut value: Value) -> Result<Value> {
    for segment in path.into_iter().rev() {
        match segment {
            KeyPathSegment::Map { key } => {
                value = Value::Map(BTreeMap::from([(key, value)]));
            }
            KeyPathSegment::Seq { index } => {
                if index == 0 {
                    value = Value::Seq(vec![value]);
                } else {
                    return Err(ConfigError::new(ConfigErrorType::ValidationError(
                        "indices other than 0 not supported when setting values with manual source"
                            .to_string(),
                    )));
                }
            }
        }
    }
    Ok(value)
}

impl<C: Config> Default for ManualSource<C> {
    fn default() -> Self {
        Self {
            value: None,
            _phantom: PhantomData,
        }
    }
}

impl<C: Config> ManualSource<C> {
    /// Creates a new manual source.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a value at the given path.
    pub fn set<K: Into<KeyPath>, V: serde::Serialize>(&mut self, path: K, value: V) -> Result<()> {
        let path: KeyPath = path.into();
        let value = serde_value::to_value(value)
            .map_err(Into::into)
            .map_err(ConfigError::new)
            .map_err(|e| e.with_source(ErrorSource::Manual))?;
        let value = C::transform(&path, value_to_value_graph(path.clone(), value)?)?;
        if let Some(old_value) = self.value.take() {
            self.value = Some(crate::merge(value, old_value));
        } else {
            self.value = Some(value);
        }
        Ok(())
    }
}

impl<C: Config> Source<C> for ManualSource<C> {
    fn get_key(&self, path: &crate::KeyPath) -> crate::Result<Option<Value>> {
        match &self.value {
            Some(value) => {
                utils::get_key::<C>(value, path).map_err(|e| e.with_source(ErrorSource::Manual))
            }
            None => Ok(None),
        }
    }
}
