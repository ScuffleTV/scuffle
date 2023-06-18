use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
};

use crate::{
    transform_from_graph, Config, ConfigError, ConfigErrorType, KeyGraph, KeyPath, KeyPathSegment,
    Result, Value,
};

pub fn graph_to_map(
    mut graph: Arc<KeyGraph>,
    key: Option<&str>,
    mut visited: HashSet<&'static str>,
) -> Result<Option<(Arc<KeyGraph>, Arc<KeyGraph>)>> {
    loop {
        match &*graph {
            KeyGraph::Map(key, value) => return Ok(Some((key.clone(), value.clone()))),
            KeyGraph::Struct(values) => {
                let key = key.ok_or_else(|| {
                    ConfigError::new(ConfigErrorType::ValidationError(format!(
                        "expected map, found {:?}",
                        graph
                    )))
                })?;

                let Some(value) = values.get(key) else {
                    return Ok(None);
                };

                return Ok(Some((
                    Arc::new(KeyGraph::String),
                    Arc::new(value.graph().clone()),
                )));
            }
            KeyGraph::Option(inner) => {
                graph = inner.clone();
            }
            KeyGraph::Ref(weak, ty) => {
                let strong = weak.upgrade().ok_or_else(|| {
                    ConfigError::new(ConfigErrorType::ValidationError(format!(
                        "weak reference to {:?} was dropped",
                        ty
                    )))
                })?;

                if !visited.insert(ty) {
                    return Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
                        "cyclic reference to {:?}",
                        ty
                    ))));
                }

                graph = strong;
            }
            _ => return Ok(None),
        }
    }
}

pub fn graph_to_seq(
    mut graph: Arc<KeyGraph>,
    mut visited: HashSet<&'static str>,
) -> Result<Option<Arc<KeyGraph>>> {
    loop {
        match &*graph {
            KeyGraph::Seq(inner) => return Ok(Some(inner.clone())),
            KeyGraph::Option(inner) => {
                graph = inner.clone();
            }
            KeyGraph::Ref(weak, ty) => {
                let strong = weak.upgrade().ok_or_else(|| {
                    ConfigError::new(ConfigErrorType::ValidationError(format!(
                        "weak reference to {:?} was dropped",
                        ty
                    )))
                })?;

                if !visited.insert(ty) {
                    return Err(ConfigError::new(ConfigErrorType::ValidationError(format!(
                        "cyclic reference to {:?}",
                        ty
                    ))));
                }

                graph = strong;
            }
            _ => return Ok(None),
        }
    }
}

pub fn value_to_map<'a>(
    path: &KeyPath,
    mut value: &'a Value,
) -> Result<Option<&'a BTreeMap<Value, Value>>> {
    loop {
        match value {
            Value::Map(map) => return Ok(Some(map)),
            Value::Option(None) => return Ok(None),
            Value::Option(Some(inner)) => {
                value = inner;
            }
            _ => {
                return Err(
                    ConfigError::new(ConfigErrorType::SubkeyOnNonMap(value.clone()))
                        .with_path(path.clone()),
                )
            }
        }
    }
}

pub fn value_to_seq<'a>(path: &KeyPath, mut value: &'a Value) -> Result<Option<&'a Vec<Value>>> {
    loop {
        match value {
            Value::Seq(seq) => return Ok(Some(seq)),
            Value::Option(None) => return Ok(None),
            Value::Option(Some(inner)) => {
                value = inner;
            }
            _ => {
                return Err(
                    ConfigError::new(ConfigErrorType::SubIndexOnNonSeq(value.clone()))
                        .with_path(path.clone()),
                )
            }
        }
    }
}

pub fn get_key<C: Config>(mut current: &Value, path: &KeyPath) -> Result<Option<Value>> {
    let mut graph = C::graph();

    for segment in path {
        match segment {
            KeyPathSegment::Map { key } => {
                let key_str = String::transform(path, key.clone())
                    .map(|key| {
                        if let Value::String(key) = key {
                            key
                        } else {
                            unreachable!();
                        }
                    })
                    .ok();

                let Some((key_graph, value_graph)) =
                    graph_to_map(graph, key_str.as_deref(), HashSet::new())?
                else {
                    return Ok(None);
                };

                let Some(map) = value_to_map(path, current)? else {
                    return Ok(None);
                };

                let key = transform_from_graph(path, &key_graph, key.clone())?;

                let Some(value) = map.get(&key) else {
                    return Ok(None);
                };

                current = value;
                graph = value_graph;
            }
            KeyPathSegment::Seq { index } => {
                let Some(seq) = value_to_seq(path, current)? else {
                    return Ok(None);
                };

                if *index >= seq.len() {
                    return Ok(None);
                }

                let Some(seq_graph) = graph_to_seq(graph, HashSet::new())? else {
                    return Ok(None);
                };

                graph = seq_graph;

                current = &seq[*index];
            }
        }
    }

    Ok(Some(current.clone()))
}
