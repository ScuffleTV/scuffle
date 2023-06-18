//! Environment source
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
//! // Add EnvSource with prefix TEST
//! builder.add_source(sources::EnvSource::with_prefix("TEST")?);
//! // Build the final configuration
//! let config: MyConfig = builder.build()?;
//! # Ok(())
//! # }
//! ```

use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};

use crate::{
    Config, ConfigError, ConfigErrorType, ErrorSource, KeyGraph, KeyPath, Result, Source, Value,
};

use super::utils;

/// Environment source
///
/// Create a new environment source with [`EnvSource::new`](EnvSource::new), [`EnvSource::with_prefix`](EnvSource::with_prefix) or [`EnvSource::with_joiner`](EnvSource::with_joiner).
pub struct EnvSource<C: Config> {
    value: Value,
    _phantom: PhantomData<C>,
}

impl<C: Config> EnvSource<C> {
    /// Creates a new environment source with no prefix and `_` as joiner.
    pub fn new() -> Result<Self> {
        Self::with_joiner(None, "_")
    }

    /// Creates a new environment source with a given prefix and `_` as joiner.
    pub fn with_prefix(prefix: &str) -> Result<Self> {
        Self::with_joiner(Some(prefix), "_")
    }

    /// Creates a new environment source with a given prefix and joiner.
    pub fn with_joiner(prefix: Option<&str>, joiner: &str) -> Result<Self> {
        Ok(Self {
            _phantom: PhantomData,
            value: extract_keys(
                &C::graph(),
                prefix,
                prefix
                    .map(|p| KeyPath::root().push_struct(p))
                    .unwrap_or_default(),
                joiner,
                false,
                false,
            )
            .and_then(|val| {
                C::transform(
                    &KeyPath::root(),
                    val.unwrap_or_else(|| Value::Map(BTreeMap::new())),
                )
            })
            .map_err(|e| {
                let e = e.with_source(ErrorSource::Env);
                if prefix.is_some() {
                    if let Some(path) = e.path().cloned() {
                        e.with_path(path.drop_root())
                    } else {
                        e
                    }
                } else {
                    e
                }
            })?,
        })
    }
}

fn extract_keys(
    graph: &KeyGraph,
    prefix: Option<&str>,
    path: KeyPath,
    joiner: &str,
    seq: bool,
    optional: bool,
) -> Result<Option<Value>> {
    match graph {
        KeyGraph::Bool
        | KeyGraph::F32
        | KeyGraph::F64
        | KeyGraph::I8
        | KeyGraph::I16
        | KeyGraph::I32
        | KeyGraph::I64
        | KeyGraph::String
        | KeyGraph::Char
        | KeyGraph::U8
        | KeyGraph::U16
        | KeyGraph::U32
        | KeyGraph::U64
        | KeyGraph::Unit => {
            let name = path
                .iter()
                .map(|s| s.to_string().to_uppercase())
                .collect::<Vec<_>>()
                .join(joiner);

            // Parse to requested type
            let Ok(value) = std::env::var(name) else {
                return Ok(None);
            };

            if optional && value.is_empty() {
                return Ok(Some(Value::Option(None)));
            }

            if seq {
                Ok(Some(Value::Seq(
                    value
                        .split(',')
                        .map(|s| Value::String(s.to_string()))
                        .collect(),
                )))
            } else {
                Ok(Some(Value::String(value)))
            }
        }
        KeyGraph::Struct(map) => {
            if seq {
                return Err(ConfigError::new(ConfigErrorType::UnsupportedType(Arc::new(
                    graph.clone(),
                )))
                .with_path(path));
            }

            let result = map
                .iter()
                .filter_map(|(child_path, key)| {
                    if key.skip_env() {
                        return None;
                    }

                    Some(
                        extract_keys(
                            key.graph(),
                            prefix,
                            path.push_struct(child_path),
                            joiner,
                            false,
                            false,
                        )
                        .map(|value| value.map(|value| (Value::String(child_path.clone()), value))),
                    )
                })
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .flatten()
                .collect::<BTreeMap<_, _>>();

            if result.is_empty() && path.get_inner().len() != prefix.is_some() as usize {
                Ok(None)
            } else {
                Ok(Some(Value::Map(result)))
            }
        }
        KeyGraph::Option(graph) => {
            if seq {
                return Err(
                    ConfigError::new(ConfigErrorType::UnsupportedType(graph.clone()))
                        .with_path(path),
                );
            }

            extract_keys(graph, prefix, path, joiner, seq, true)
        }
        KeyGraph::Seq(graph) => {
            if seq {
                return Err(
                    ConfigError::new(ConfigErrorType::UnsupportedType(graph.clone()))
                        .with_path(path),
                );
            }

            extract_keys(graph, prefix, path, joiner, true, false)
        }
        KeyGraph::Map(_, _) => Err(ConfigError::new(ConfigErrorType::UnsupportedType(Arc::new(
            graph.clone(),
        )))
        .with_path(path)),
        KeyGraph::Ref(_, _) => Err(ConfigError::new(ConfigErrorType::UnsupportedType(Arc::new(
            graph.clone(),
        )))
        .with_path(path)),
    }
}

impl<C: Config> Source<C> for EnvSource<C> {
    fn get_key(&self, path: &KeyPath) -> Result<Option<Value>> {
        utils::get_key::<C>(&self.value, path).map_err(|e| e.with_source(ErrorSource::Env))
    }
}
