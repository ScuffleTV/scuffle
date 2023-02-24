use async_graphql::{ErrorExtensions, Value};

use crate::api::v1::gql::error::{GqlError, Result, ResultExt};

#[test]
fn test_error_from_residual_error() {
    let fn1 = || -> Result<()> {
        if true {
            Err(anyhow::anyhow!("error from fn1"))
        } else {
            Ok(())
        }
        .extend_gql("error somewhere")?;
        Ok(())
    };
    let err = fn1().unwrap_err();
    let err = err.extend();
    assert_eq!(
        err.message,
        format!("{}: error somewhere", GqlError::InternalServerError)
    );
    assert!(err.source.is_none());
    assert!(err.extensions.is_some());
    let extensions = err.extensions.unwrap();
    assert_eq!(
        extensions.get("kind"),
        Some(&Value::from(format!("{}", GqlError::InternalServerError)))
    );
    assert_eq!(
        extensions.get("reason"),
        Some(&Value::from("error somewhere".to_string()))
    );
}
