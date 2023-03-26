use std::time::Duration;

use common::prelude::FutureTimeout;
use hyper::StatusCode;

use crate::{
    api::run,
    config::{ApiConfig, AppConfig},
    tests::global::mock_global_state,
};

mod errors;
mod v1;

#[tokio::test]
async fn test_api_v6() {
    let port = portpicker::pick_unused_port().expect("failed to pick port");
    let (global, handler) = mock_global_state(AppConfig {
        api: ApiConfig {
            bind_address: format!("[::]:{}", port).parse().unwrap(),
            tls: None,
        },
        ..Default::default()
    })
    .await;

    let handle = tokio::spawn(run(global));

    // We need to wait for the server to start
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://localhost:{}/v1/health", port))
        .send()
        .await
        .expect("failed to get health");

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.text().await.expect("failed to read body");
    assert_eq!(body, "{\"status\":\"ok\"}");

    // The client uses Keep-Alive, so we need to drop it to release the global context
    drop(client);

    handler
        .cancel()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel context");
    handle
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel api")
        .expect("api failed")
        .expect("api failed");
}

#[tokio::test]
async fn test_api_v4() {
    let port = portpicker::pick_unused_port().expect("failed to pick port");

    let (global, handler) = mock_global_state(AppConfig {
        api: ApiConfig {
            bind_address: format!("0.0.0.0:{}", port).parse().unwrap(),
            tls: None,
        },
        ..Default::default()
    })
    .await;

    let handle = tokio::spawn(run(global));

    // We need to wait for the server to start
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://localhost:{}/v1/health", port))
        .send()
        .await
        .expect("failed to get health");

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.text().await.expect("failed to read body");
    assert_eq!(body, "{\"status\":\"ok\"}");

    // The client uses Keep-Alive, so we need to drop it to release the global context
    drop(client);

    handler
        .cancel()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel context");
    handle
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel api")
        .expect("api failed")
        .expect("api failed");
}
