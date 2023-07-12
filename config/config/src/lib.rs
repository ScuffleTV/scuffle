#![doc = include_str!("../README.md")]

use std::collections::{btree_map, BTreeMap, BTreeSet};
use std::iter;
use std::ops::Deref;
use std::sync::Arc;

pub use config_derive::Config;

mod error;
mod key;
mod primitives;
pub mod sources;

pub use error::*;
pub use key::*;
pub use primitives::*;
use serde_ignored::Path;
pub use serde_value::Value;
use sources::ManualSource;

/// Config source
///
/// Every type that implements this trait can be added as a source to a [`ConfigBuilder`](ConfigBuilder).
pub trait Source<C: Config> {
    /// Gets a single key by its path.
    fn get_key(&self, path: &KeyPath) -> Result<Option<Value>>;
}

pub fn parse_key<'de, D: serde::de::Deserialize<'de>>(value: Value) -> Result<D> {
    let mut paths = BTreeSet::new();

    let mut cb = |path: Path| {
        paths.insert(path.to_string());
    };

    let ignored_de = serde_ignored::Deserializer::new(value, &mut cb);

    let value = match serde_path_to_error::deserialize(ignored_de) {
        Ok(t) => Ok(t),
        Err(e) => {
            let path = e.path().to_string();
            let error = e.into_inner();
            Err(ConfigError::new(error.into()).with_path(path.as_str().into()))
        }
    };

    if !paths.is_empty() {
        tracing::warn!(
            "fields ignored while deserializing, {}",
            paths.into_iter().collect::<Vec<_>>().join(", ")
        );
    }

    value
}

/// This is the main trait of this crate.
///
/// Every type that implements this trait can be parsed as a config and any type that is part of a config needs to implement this trait.
/// Typically you want use the derive macro to implement it.
pub trait Config: serde::de::DeserializeOwned + 'static {
    const PKG_NAME: Option<&'static str> = None;
    const ABOUT: Option<&'static str> = None;
    const VERSION: Option<&'static str> = None;
    const AUTHOR: Option<&'static str> = None;

    /// Returns the [`KeyGraph`](KeyGraph) of this config type.
    fn graph() -> Arc<KeyGraph>;

    /// TODO
    fn transform(path: &KeyPath, value: Value) -> Result<Value> {
        transform_from_graph(path, &Self::graph(), value)
    }
}

pub fn transform_from_graph(path: &KeyPath, graph: &KeyGraph, value: Value) -> Result<Value> {
    match graph {
        KeyGraph::Bool => bool::transform(path, value),
        KeyGraph::String => String::transform(path, value),
        KeyGraph::Char => char::transform(path, value),
        KeyGraph::I64 => i64::transform(path, value),
        KeyGraph::U64 => u64::transform(path, value),
        KeyGraph::I32 => i32::transform(path, value),
        KeyGraph::U32 => u32::transform(path, value),
        KeyGraph::I16 => i16::transform(path, value),
        KeyGraph::U16 => u16::transform(path, value),
        KeyGraph::I8 => i8::transform(path, value),
        KeyGraph::U8 => u8::transform(path, value),
        KeyGraph::F32 => f32::transform(path, value),
        KeyGraph::F64 => f64::transform(path, value),
        KeyGraph::Unit => <()>::transform(path, value),
        KeyGraph::Option(graph) => {
            if let Value::Option(value) = value {
                if let Some(value) = value {
                    transform_from_graph(path, graph, *value)
                } else {
                    Ok(Value::Option(None))
                }
            } else {
                transform_from_graph(path, graph, value)
            }
        }
        KeyGraph::Seq(graph) => {
            if let Value::Seq(seq) = value {
                let mut result = Vec::new();
                for (idx, value) in seq.into_iter().enumerate() {
                    let value = transform_from_graph(&path.push_seq(idx), graph, value)?;
                    result.push(value);
                }

                Ok(Value::Seq(result))
            } else {
                Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
                    "expected sequence, found {:?}",
                    value
                )))
                .with_path(path.clone()))
            }
        }
        KeyGraph::Map(key_graph, value_graph) => {
            if let Value::Map(map) = value {
                let mut result = BTreeMap::new();
                for (key, value) in map {
                    let key = transform_from_graph(&path.push_map(&key), key_graph, key)?;
                    let value = transform_from_graph(&path.push_map(&key), value_graph, value)?;
                    result.insert(key, value);
                }

                Ok(Value::Map(result))
            } else {
                Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
                    "expected map, found {:?}",
                    value
                )))
                .with_path(path.clone()))
            }
        }
        KeyGraph::Struct(tree) => {
            if let Value::Map(map) = value {
                let mut result = BTreeMap::new();
                for (key, value) in map {
                    let key = String::transform(path, key)?;
                    let key_str = if let Value::String(key) = key {
                        key
                    } else {
                        return Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
                            "expected string, found {:?}",
                            key
                        )))
                        .with_path(path.clone()));
                    };

                    let key_tree = if let Some(key_tree) = tree.get(&key_str) {
                        key_tree
                    } else {
                        // We dont know what key this is so we will let serde ignore it.
                        result.insert(Value::String(key_str), value);
                        continue;
                    };

                    let value = if let Some(validator) = key_tree.transformer() {
                        validator(&path.push_struct(&key_str), value)?
                    } else {
                        transform_from_graph(&path.push_struct(&key_str), key_tree.graph(), value)?
                    };

                    result.insert(Value::String(key_str), value);
                }

                Ok(Value::Map(result))
            } else {
                Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
                    "expected map, found {:?}",
                    value
                )))
                .with_path(path.clone()))
            }
        }
        KeyGraph::Ref(graph, ty) => {
            let graph = graph
                .upgrade()
                .ok_or_else(|| ConfigErrorType::InvalidReference(ty))?;
            transform_from_graph(path, &graph, value)
        }
    }
    .map_err(|e| e.with_path(path.clone()))
}

struct SourceHolder<C: Config> {
    source: Box<dyn Source<C>>,
    // The higher the priority, the earlier the source will be checked
    priority: usize,
}

impl<C: Config> SourceHolder<C> {
    pub fn new(source: impl Source<C> + 'static, priority: usize) -> Self {
        Self {
            source: Box::new(source),
            priority,
        }
    }
}

impl<C: Config> Deref for SourceHolder<C> {
    type Target = dyn Source<C>;

    fn deref(&self) -> &Self::Target {
        &*self.source
    }
}

/// Use this struct to add sources and build a config.
pub struct ConfigBuilder<C: Config> {
    sources: Vec<SourceHolder<C>>,
    overwrite: ManualSource<C>,
}

impl<C: Config> Default for ConfigBuilder<C> {
    fn default() -> Self {
        Self::new()
    }
}

fn merge(first: Value, second: Value) -> Value {
    match (first, second) {
        (Value::Map(first), Value::Map(mut second)) => {
            for (k1, v1) in first {
                match second.entry(k1) {
                    btree_map::Entry::Vacant(entry) => {
                        entry.insert(v1);
                    }
                    btree_map::Entry::Occupied(entry) => {
                        let (k2, v2) = entry.remove_entry();
                        second.insert(k2, merge(v1, v2));
                    }
                }
            }
            Value::Map(second)
        }
        (Value::Seq(first), Value::Seq(second)) => {
            let mut merged = Vec::with_capacity(std::cmp::max(first.len(), second.len()));
            let mut first = first.into_iter();
            let mut second = second.into_iter();

            loop {
                match (first.next(), second.next()) {
                    (None, None) => break,
                    (Some(first), Some(second)) => merged.push(merge(first, second)),
                    (None, Some(second)) => merged.push(second),
                    (Some(first), None) => merged.push(first),
                }
            }

            Value::Seq(merged)
        }
        (first, _) => first,
    }
}

impl<C: Config> ConfigBuilder<C> {
    /// Creates a new config builder with no sources.
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            overwrite: ManualSource::new(),
        }
    }

    /// Adds a source to the config builder.
    ///
    /// This is the same as calling [`add_source_with_priority`](ConfigBuilder::add_source_with_priority) with a priority of 0.
    pub fn add_source<S: Source<C> + 'static>(&mut self, source: S) -> &mut Self {
        self.add_source_with_priority(source, 0)
    }

    /// Adds a source to the config builder with a priority.
    ///
    /// The ealier a source is added and the higher its priority is, the higher its importance is.
    /// This means that values from sources with a lower importance will **not** overwrite values from sources with a higher importance.
    pub fn add_source_with_priority<S: Source<C> + 'static>(
        &mut self,
        source: S,
        priority: usize,
    ) -> &mut Self {
        self.sources.push(SourceHolder::new(source, priority));
        self.sort_sources();

        self
    }

    fn sort_sources(&mut self) {
        self.sources.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Overwrites a single key.
    ///
    /// This means that all values for this key that come from the added sources will be ignored.
    pub fn overwrite<K: Into<KeyPath>, V: serde::Serialize>(
        &mut self,
        key: K,
        value: V,
    ) -> Result<()> {
        self.overwrite.set(key.into(), value)
    }

    /// Gets a single key by its path.
    pub fn get_key(&self, path: impl Into<KeyPath>) -> Result<Option<Value>> {
        let key_path = path.into();

        let mut iter = iter::once(self.overwrite.get_key(&key_path))
            .chain(self.sources.iter().map(|s| s.get_key(&key_path)))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten();

        let Some(mut value) = iter.next() else {
            return Ok(None);
        };

        for v in iter {
            value = merge(value, v);
        }

        Ok(Some(value))
    }

    /// Parses a single key.
    pub fn parse_key<'de, T: serde::de::Deserialize<'de>>(
        &self,
        path: impl Into<KeyPath>,
    ) -> Result<T> {
        // We can use serde_ignored to find more information about the state of our struct.
        let value = self.get_key(path)?.unwrap_or(Value::Option(None));

        parse_key(value)
    }

    /// Builds the config.
    ///
    /// This function iterates all added sources and gets each key that is required to build `C`.
    ///
    /// After that it will deserialize the values into `C` using serde.
    pub fn build(&self) -> Result<C> {
        self.parse_key::<C>(KeyPath::root())
    }
}
