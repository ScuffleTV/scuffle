use std::time::Duration;

use hyper::StatusCode;

use crate::{api::run, config::AppConfig, tests::global::mock_global_state};

mod errors;
mod v1;

#[tokio::test]
async fn test_api_v6() {
    let (global, handler) = mock_global_state(AppConfig {
        bind_address: "[::]:8081".to_string(),
        ..Default::default()
    })
    .await;

    let handle = tokio::spawn(run(global));

    // We need to wait for the server to start
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = reqwest::Client::new();
    let resp = client
        .get("http://localhost:8081/v1/health")
        .send()
        .await
        .expect("failed to get health");

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.text().await.expect("failed to read body");
    assert_eq!(body, "{\"status\":\"ok\"}");

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
    let (global, handler) = mock_global_state(AppConfig {
        bind_address: "0.0.0.0:8081".to_string(),
        ..Default::default()
    })
    .await;

    let handle = tokio::spawn(run(global));

    // We need to wait for the server to start
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = reqwest::Client::new();
    let resp = client
        .get("http://localhost:8081/v1/health")
        .send()
        .await
        .expect("failed to get health");

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.text().await.expect("failed to read body");
    assert_eq!(body, "{\"status\":\"ok\"}");

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
    let (global, handler) = mock_global_state(AppConfig {
        bind_address: "???".to_string(),
        ..Default::default()
    })
    .await;

    assert!(run(global).await.is_err());

    tokio::time::timeout(Duration::from_secs(1), handler.cancel())
        .await
        .expect("failed to cancel context");
}
