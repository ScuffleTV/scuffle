use async_graphql::{InputValueError, InputValueResult, Scalar, ScalarType, Value};
use chrono::{TimeZone, Utc};

#[derive(Clone, Debug)]
pub struct DateRFC3339(pub chrono::DateTime<Utc>);

#[Scalar]
impl ScalarType for DateRFC3339 {
    fn parse(value: Value) -> InputValueResult<Self> {
        match value {
            Value::String(s) => match chrono::DateTime::parse_from_rfc3339(&s) {
                Ok(dt) => {
                    let dt = dt.with_timezone(&Utc);
                    Ok(DateRFC3339(dt))
                }
                Err(e) => Err(InputValueError::custom(e.to_string())),
            },
            Value::Number(n) => match n.as_i64() {
                Some(n) => {
                    let dt = Utc.timestamp_opt(n, 0);
                    let dt = dt
                        .single()
                        .ok_or(InputValueError::custom("Invalid number"))?;
                    Ok(DateRFC3339(dt))
                }
                None => Err(InputValueError::custom("Invalid number")),
            },
            _ => Err(InputValueError::custom("Invalid value")),
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.0.to_rfc3339())
    }
}

impl From<chrono::DateTime<Utc>> for DateRFC3339 {
    fn from(dt: chrono::DateTime<Utc>) -> Self {
        DateRFC3339(dt)
    }
}
