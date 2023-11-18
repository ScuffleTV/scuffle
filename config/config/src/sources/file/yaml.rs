use std::collections::BTreeMap;

use crate::{ConfigError, ConfigErrorType, KeyPath, Result, Value};

pub fn convert_value(path: &KeyPath, value: serde_yaml::Value) -> Result<Value> {
	match value {
		serde_yaml::Value::String(s) => Ok(Value::String(s)),
		serde_yaml::Value::Number(n) => Ok(n
			.as_i64()
			.map(Value::I64)
			.or_else(|| n.as_u64().map(Value::U64))
			.unwrap_or_else(|| Value::F64(n.as_f64().unwrap()))),
		serde_yaml::Value::Bool(b) => Ok(Value::Bool(b)),
		serde_yaml::Value::Sequence(a) => Ok(Value::Seq(
			a.into_iter()
				.enumerate()
				.map(|(idx, value)| convert_value(&path.push_seq(idx), value))
				.collect::<Result<_>>()?,
		)),
		serde_yaml::Value::Mapping(map) => {
			let mut hashmap = BTreeMap::new();
			for (k, v) in map {
				let key = convert_value(path, k)?;
				let value = convert_value(&path.push_map(&key), v)?;
				hashmap.insert(key, value);
			}
			Ok(Value::Map(hashmap))
		}
		serde_yaml::Value::Null => Ok(Value::Option(None)),
		serde_yaml::Value::Tagged(_) => Err(ConfigError::new(ConfigErrorType::DeserializedTypeNotSupported(
			"tagged yaml values".to_string(),
		))
		.with_path(path.clone())),
	}
}
