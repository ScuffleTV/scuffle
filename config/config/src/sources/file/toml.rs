use std::collections::BTreeMap;

use crate::{ConfigError, ConfigErrorType, KeyPath, Result, Value};

pub fn convert_value(path: &KeyPath, value: toml::Value) -> Result<Value> {
	match value {
		toml::Value::String(s) => Ok(Value::String(s)),
		toml::Value::Integer(i) => Ok(Value::I64(i)),
		toml::Value::Float(f) => Ok(Value::F64(f)),
		toml::Value::Boolean(b) => Ok(Value::Bool(b)),
		toml::Value::Array(a) => Ok(Value::Seq(
			a.into_iter()
				.enumerate()
				.map(|(idx, value)| convert_value(&path.push_seq(idx), value))
				.collect::<Result<Vec<_>>>()?,
		)),
		toml::Value::Table(map) => {
			// Is there a better way than iterating over each entry? Probably not
			let mut hashmap = BTreeMap::new();
			for (key, value) in map {
				let path = path.push_struct(&key);
				hashmap.insert(Value::String(key), convert_value(&path, value)?);
			}
			Ok(Value::Map(hashmap))
		}
		_ => Err(
			ConfigError::new(ConfigErrorType::DeserializedTypeNotSupported(value.type_str().to_string()))
				.with_path(path.clone()),
		),
	}
}
