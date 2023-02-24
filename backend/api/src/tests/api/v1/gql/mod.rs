use std::time::Duration;

use async_graphql::http::WebSocketProtocols;
use futures_util::{SinkExt, StreamExt};
use http::HeaderValue;
use hyper_tungstenite::tungstenite::client::IntoClientRequest;
use serde_json::json;

use crate::{
    api,
    api::v1::gql::{schema, PLAYGROUND_HTML},
    config::AppConfig,
    tests::global::mock_global_state,
};

mod auth;
mod errors;
mod models;

#[tokio::test]
async fn test_query_noop() {
    let schema = schema();
    let query = r#"
        query {
            noop
        }
    "#;
    let res = schema.execute(query).await;
    assert_eq!(res.errors.len(), 0);
    let json = res.data.into_json();
    assert!(json.is_ok());
    assert_eq!(json.unwrap(), serde_json::json!({ "noop": false }));
}

#[tokio::test]
async fn test_subscription_noop() {
    let schema = schema();
    let query = r#"
        subscription {
            noop
        }
    "#;
    let mut res = schema.execute_stream(query);
    let resp = res.next().await;
    assert!(resp.is_none());
}

#[tokio::test]
async fn test_query_noop_via_http() {
    let (global, handler) = mock_global_state(AppConfig {
        bind_address: "0.0.0.0:8081".to_string(),
        ..Default::default()
    })
    .await;

    let h = tokio::spawn(api::run(global));

    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = reqwest::Client::new();
    let res = client
        .post("http://localhost:8081/v1/gql")
        .json(&serde_json::json!({
            "query": "query { noop }",
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 200);
    let json: serde_json::Value = res.json().await.unwrap();
    assert_eq!(
        json.get("data"),
        Some(&serde_json::json!({ "noop": false } ))
    );

    let res = client
        .get("http://localhost:8081/v1/gql")
        .query(&[("query", "query { noop }")])
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 200);
    let json: serde_json::Value = res.json().await.unwrap();
    assert_eq!(
        json.get("data"),
        Some(&serde_json::json!({ "noop": false } ))
    );

    drop(client);

    // Connect via websocket
    let mut req = "ws://localhost:8081/v1/gql".into_client_request().unwrap();
    req.headers_mut().insert(
        http::header::SEC_WEBSOCKET_PROTOCOL,
        HeaderValue::from_static(WebSocketProtocols::GraphQLWS.sec_websocket_protocol()),
    );

    let (mut ws_stream, resp) = tokio_tungstenite::connect_async(req).await.unwrap();

    assert_eq!(resp.status(), 101);
    assert_eq!(
        resp.headers().get("sec-websocket-protocol"),
        Some(&HeaderValue::from_static(
            WebSocketProtocols::GraphQLWS.sec_websocket_protocol()
        ))
    );

    // Send a message
    let msg = serde_json::json!({
        "type": "connection_init",
        "payload": {}
    });

    ws_stream
        .send(tokio_tungstenite::tungstenite::Message::Text(
            serde_json::to_string(&msg).unwrap(),
        ))
        .await
        .unwrap();

    // Receive a message
    let msg = serde_json::from_str::<serde_json::Value>(
        ws_stream
            .next()
            .await
            .unwrap()
            .unwrap()
            .to_string()
            .as_str(),
    )
    .unwrap();
    assert_eq!(
        msg,
        json!({
            "type": "connection_ack",
        })
    );

    // Send a message
    let msg = serde_json::json!({"id":"bc491f76-500b-41c2-b6c2-3dd1274f3baa","type":"subscribe","payload":{"query":"subscription {\n  noop\n}"}});

    ws_stream
        .send(tokio_tungstenite::tungstenite::Message::Text(
            serde_json::to_string(&msg).unwrap(),
        ))
        .await
        .unwrap();

    // Receive a message
    let msg = serde_json::from_str::<serde_json::Value>(
        ws_stream
            .next()
            .await
            .unwrap()
            .unwrap()
            .to_string()
            .as_str(),
    )
    .unwrap();
    assert_eq!(
        msg,
        json!({
            "id": "bc491f76-500b-41c2-b6c2-3dd1274f3baa",
            "type": "complete",
        })
    );

    // Close the connection
    ws_stream
        .send(tokio_tungstenite::tungstenite::Message::Close(None))
        .await
        .unwrap();

    // Wait for the connection to close
    ws_stream.next().await;

    // Wait for the server to shutdown
    tokio::time::timeout(std::time::Duration::from_secs(1), handler.cancel())
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(1), h)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn test_playground() {
    let (global, handler) = mock_global_state(AppConfig {
        bind_address: "0.0.0.0:8081".to_string(),
        ..Default::default()
    })
    .await;

    let h = tokio::spawn(api::run(global));

    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = reqwest::Client::new();
    let res = client
        .get("http://localhost:8081/v1/gql/playground")
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 200);
    assert_eq!(
        res.headers().get("content-type").unwrap().to_str().unwrap(),
        "text/html"
    );
    let text = res.text().await.unwrap();
    assert_eq!(text, PLAYGROUND_HTML);

    drop(client);

    // Wait for the server to shutdown
    tokio::time::timeout(std::time::Duration::from_secs(1), handler.cancel())
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(1), h)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
