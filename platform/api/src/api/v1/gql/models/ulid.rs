use std::ops::Deref;

use async_graphql::{Description, InputValueError, InputValueResult, Scalar, ScalarType, Value};

/// A ULID (Universally Unique Lexicographically Sortable Identifier) scalar.
#[derive(Copy, Clone, Debug, Description)]
pub struct GqlUlid(ulid::Ulid);

impl GqlUlid {
    pub fn to_ulid(self) -> ulid::Ulid {
        self.0
    }

    pub fn to_uuid(self) -> uuid::Uuid {
        self.0.into()
    }
}

#[Scalar(
    name = "ULID",
    specified_by_url = "https://github.com/ulid/spec",
    use_type_description
)]
impl ScalarType for GqlUlid {
    fn parse(value: Value) -> InputValueResult<Self> {
        match value {
            Value::String(s) => match ulid::Ulid::from_string(&s) {
                Ok(ulid) => Ok(GqlUlid(ulid)),
                Err(e) => Err(InputValueError::custom(e.to_string())),
            },
            // Can't support integer values here because async-graphql doesn't support 128-bit integers.
            _ => Err(InputValueError::custom("Invalid value")),
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.0.to_string())
    }
}

impl Deref for GqlUlid {
    type Target = ulid::Ulid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ulid::Ulid> for GqlUlid {
    fn from(value: ulid::Ulid) -> Self {
        Self(value)
    }
}

impl From<uuid::Uuid> for GqlUlid {
    fn from(value: uuid::Uuid) -> Self {
        Self::from(ulid::Ulid::from(value))
    }
}

impl From<GqlUlid> for ulid::Ulid {
    fn from(value: GqlUlid) -> Self {
        value.0
    }
}
