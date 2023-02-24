use std::error::Error;

use http::StatusCode;
use hyper::{
    body::{Bytes, HttpBody},
    Body,
};

use crate::api::error::{Result, ResultExt, ShouldLog};

#[test]
fn test_error_from_residual_string() {
    let fn1 = || -> Result<()> {
        if true { Err("error from fn1") } else { Ok(()) }?;

        Ok(())
    };

    let err = fn1().unwrap_err();

    assert_eq!(err.should_log(), ShouldLog::Yes);
    assert_eq!(err.location().file(), file!());
    assert_eq!(err.response().status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn test_error_from_residual_response() {
    let fn1 = || -> Result<()> {
        if true {
            Err(hyper::Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::empty())
                .unwrap())
        } else {
            Ok(())
        }?;

        Ok(())
    };

    let err = fn1().unwrap_err();

    assert_eq!(err.should_log(), ShouldLog::No);
    assert_eq!(err.location().file(), file!());
    assert_eq!(err.response().status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_error_from_residual_tuple() {
    let fn1 = || -> Result<()> {
        if true {
            Err((StatusCode::CONFLICT, "error from fn1"))
        } else {
            Ok(())
        }?;

        Ok(())
    };

    let err = fn1().unwrap_err();

    assert_eq!(err.should_log(), ShouldLog::No);
    assert_eq!(err.location().file(), file!());
    assert_eq!(err.response().status(), StatusCode::CONFLICT);
}

#[test]
fn test_error_from_residual_tuple_with_error() {
    let fn1 = || -> Result<()> {
        if true {
            Err((
                StatusCode::CONFLICT,
                "error from fn1",
                anyhow::anyhow!("error from fn1"),
            ))
        } else {
            Ok(())
        }?;

        Ok(())
    };

    let err = fn1().unwrap_err();

    assert_eq!(err.should_log(), ShouldLog::Debug);
    assert_eq!(err.location().file(), file!());
    assert_eq!(err.response().status(), StatusCode::CONFLICT);
}

#[test]
fn test_error_from_residual_error() {
    let fn1 = || -> Result<()> {
        if true {
            Err(anyhow::anyhow!("error from fn1"))
        } else {
            Ok(())
        }
        .extend_route("failed somewhere")?;

        Ok(())
    };

    let err = fn1().unwrap_err();

    assert_eq!(err.should_log(), ShouldLog::Yes);
    assert_eq!(err.location().file(), file!());
    assert_eq!(err.response().status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn test_error_debug_display() {
    let fn1 = || -> Result<()> {
        if true {
            Err(anyhow::anyhow!("error from fn1"))
        } else {
            Ok(())
        }
        .extend_route("failed somewhere")?;

        Ok(())
    };

    let err = fn1().unwrap_err();

    assert_eq!(format!("{:?}", err), "RouteError: error from fn1");
    assert_eq!(format!("{}", err), "RouteError: error from fn1");
}

#[test]
fn test_error_debug_display2() {
    let fn1 = || -> Result<()> {
        if true { Err("error from fn1") } else { Ok(()) }?;

        Ok(())
    };

    let err = fn1().unwrap_err();

    assert_eq!(format!("{:?}", err), "RouteError: Unknown Source");
    assert_eq!(format!("{}", err), "RouteError: Unknown Source");
}

#[test]
fn test_std_error() {
    let fn1 = || -> Result<()> {
        if true {
            Err(anyhow::anyhow!("error from fn1"))
        } else {
            Ok(())
        }
        .extend_route("failed somwehere")?;

        Ok(())
    };

    let err = fn1().unwrap_err();

    assert_eq!(err.source().unwrap().to_string(), "error from fn1");
}

#[tokio::test]
async fn test_hyper_response() {
    let fn1 = || -> Result<()> {
        if true {
            Err(hyper::Response::builder()
                .status(StatusCode::IM_A_TEAPOT)
                .body(Body::from("raw body response"))
                .unwrap())
        } else {
            Ok(())
        }?;

        Ok(())
    };

    let err = fn1().unwrap_err();
    let mut resp = err.response();
    assert_eq!(resp.status(), StatusCode::IM_A_TEAPOT);
    assert_eq!(
        resp.body_mut().data().await.unwrap().unwrap(),
        Bytes::from("raw body response")
    );
}
