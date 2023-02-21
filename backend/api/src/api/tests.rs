use std::time::Duration;

use common::{context::Context, logging};
use hyper::{Client, StatusCode};

use crate::config::AppConfig;

use super::*;

#[tokio::test]
async fn test_api_v6() {
    let db = sqlx::PgPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL not set"))
        .await
        .expect("failed to connect to database");

    // We need to initalize logging
    logging::init("api=debug").expect("failed to initialize logging");

    let (ctx, handler) = Context::new();

    let global = Arc::new(GlobalState {
        config: AppConfig {
            bind_address: "[::]:8081".to_string(),
            database_url: "".to_string(),
            log_level: "api=debug".to_string(),
            config_file: "".to_string(),
        },
        ctx,
        db,
    });

    let handle = tokio::spawn(run(global));

    // We need to wait for the server to start
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = Client::new();

    let resp = client
        .get(
            "http://localhost:8081/v1/health"
                .to_string()
                .parse()
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = hyper::body::to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(body, "OK");

    // The client uses Keep-Alive, so we need to drop it to release the global context
    drop(client);

    tokio::time::timeout(Duration::from_secs(1), handler.cancel())
        .await
        .expect("failed to cancel context");
    tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .expect("failed to cancel api")
        .expect("api failed")
        .expect("api failed");
}

#[tokio::test]
async fn test_api_v4() {
    let db = sqlx::PgPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL not set"))
        .await
        .expect("failed to connect to database");

    // We need to initalize logging
    logging::init("api=debug").expect("failed to initialize logging");

    let (ctx, handler) = Context::new();

    let global = Arc::new(GlobalState {
        config: AppConfig {
            bind_address: "0.0.0.0:8081".to_string(),
            database_url: "".to_string(),
            log_level: "api=debug".to_string(),
            config_file: "".to_string(),
        },
        ctx,
        db,
    });

    let handle = tokio::spawn(run(global));

    // We need to wait for the server to start
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = Client::new();

    let resp = client
        .get("http://localhost:8081/v1/health".parse().unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = hyper::body::to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(body, "OK");

    // The client uses Keep-Alive, so we need to drop it to release the global context
    drop(client);

    tokio::time::timeout(Duration::from_secs(1), handler.cancel())
        .await
        .expect("failed to cancel context");
    tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .expect("failed to cancel api")
        .expect("api failed")
        .expect("api failed");
}

#[tokio::test]
async fn test_api_bad_bind() {
    let db = sqlx::PgPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL not set"))
        .await
        .expect("failed to connect to database");

    // We need to initalize logging
    logging::init("api=debug").expect("failed to initialize logging");

    let (ctx, handler) = Context::new();

    let global = Arc::new(GlobalState {
        config: AppConfig {
            bind_address: "????".to_string(),
            database_url: "".to_string(),
            log_level: "api=debug".to_string(),
            config_file: "".to_string(),
        },
        ctx,
        db,
    });

    assert!(run(global).await.is_err());

    tokio::time::timeout(Duration::from_secs(1), handler.cancel())
        .await
        .expect("failed to cancel context");
}
