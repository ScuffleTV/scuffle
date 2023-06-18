use std::collections::BTreeMap;

use crate::{KeyPath, Result, Value};

pub fn convert_value(path: &KeyPath, value: serde_json::Value) -> Result<Value> {
    match value {
        serde_json::Value::String(s) => Ok(Value::String(s)),
        serde_json::Value::Number(n) => Ok(n
            .as_i64()
            .map(Value::I64)
            .or_else(|| n.as_u64().map(Value::U64))
            .unwrap_or_else(|| Value::F64(n.as_f64().unwrap()))),
        serde_json::Value::Bool(b) => Ok(Value::Bool(b)),
        serde_json::Value::Array(a) => Ok(Value::Seq(
            a.into_iter()
                .enumerate()
                .map(|(idx, value)| convert_value(&path.push_seq(idx), value))
                .collect::<Result<Vec<_>>>()?,
        )),
        serde_json::Value::Object(map) => {
            // Is there a better way than iterating over each entry? Probably not
            let mut hashmap = BTreeMap::new();
            for (k, v) in map {
                let path = path.push_struct(&k);
                hashmap.insert(Value::String(k), convert_value(&path, v)?);
            }
            Ok(Value::Map(hashmap))
        }
        serde_json::Value::Null => Ok(Value::Option(None)),
    }
}
