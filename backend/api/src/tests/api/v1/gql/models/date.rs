use async_graphql::{ScalarType, Value};
use chrono::{Timelike, Utc};

use crate::api::v1::gql::models::date::DateRFC3339;

#[test]
fn test_date_scalar() {
    let now = Utc::now();

    let date = DateRFC3339::from(now);

    assert_eq!(ScalarType::to_value(&date), Value::from(now.to_rfc3339()));

    let date = DateRFC3339::parse(Value::from(now.to_rfc3339())).unwrap();
    assert_eq!(date.0, now);

    let date = DateRFC3339::parse(Value::from(now.timestamp())).unwrap();
    assert_eq!(date.0, now.with_nanosecond(0).unwrap()); // Nanoscends are lost in the conversion
}
