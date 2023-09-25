use crate::database::{session, user};
use chrono::{Duration, Utc};
use common::prelude::FutureTimeout;
use core::time;
use http::header;
use serde_json::{json, Value};
use serial_test::serial;

use crate::{
    api::{self, v1::jwt::JwtState},
    config::{ApiConfig, AppConfig},
    tests::global::mock_global_state,
};

#[serial]
#[tokio::test]

async fn test_serial_auth_middleware() {
    let port = portpicker::pick_unused_port().expect("failed to pick port");
    let (global, handler) = mock_global_state(AppConfig {
        api: ApiConfig {
            bind_address: format!("0.0.0.0:{}", port).parse().unwrap(),
            tls: None,
        },
        ..Default::default()
    })
    .await;

    sqlx::query!("DELETE FROM users")
        .execute(global.db.as_ref())
        .await
        .expect("failed to clear users");
    let id = sqlx::query!(
        "INSERT INTO users (username, display_name, email, password_hash, stream_key) VALUES ($1, $1, $2, $3, $4) RETURNING id",
        "test",
        "test@test.com",
        user::hash_password("test"),
        user::generate_stream_key(),
    )
    .map(|row| row.id)
    .fetch_one(global.db.as_ref())
    .await
    .expect("failed to insert user");

    let session = sqlx::query_as!(
        session::Model,
        "INSERT INTO sessions (user_id, expires_at) VALUES ($1, $2) RETURNING *",
        id,
        Utc::now() + Duration::seconds(30)
    )
    .fetch_one(global.db.as_ref())
    .await
    .expect("failed to insert session");

    let session_id = session.id;

    let token = JwtState::from(session)
        .serialize(&global)
        .expect("failed to create token");

    let handle = tokio::spawn(api::run(global.clone()));

    // We need to wait for the server to start
    tokio::time::sleep(time::Duration::from_millis(300)).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://localhost:{}/v1/health", port))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .await
        .expect("failed to get health");

    assert_eq!(resp.status(), http::StatusCode::OK);
    let body: Value = resp.json().await.expect("failed to read body");
    assert_eq!(body, json!({"status": "ok"}));

    sqlx::query!(
        "UPDATE sessions SET invalidated_at = NOW() WHERE id = $1",
        session_id
    )
    .execute(global.db.as_ref())
    .await
    .expect("failed to update session");

    let resp = client
        .get(format!("http://localhost:{}/v1/health", port))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .await
        .expect("failed to get health");

    assert_eq!(resp.status(), http::StatusCode::OK);
    assert_eq!(
        resp.headers()
            .get("X-Auth-Token-Check-Status")
            .and_then(|s| s.to_str().ok()),
        Some("failed")
    );
    let body: Value = resp.json().await.expect("failed to read body");
    assert_eq!(body, json!({"status": "ok"}));

    // The client uses Keep-Alive, so we need to drop it to release the global context
    drop(global);
    drop(client);

    handler
        .cancel()
        .timeout(time::Duration::from_secs(1))
        .await
        .expect("failed to cancel context");

    handle
        .timeout(time::Duration::from_secs(1))
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

#[serial]
#[tokio::test]
async fn test_serial_auth_middleware_failed() {
    let port = portpicker::pick_unused_port().expect("failed to pick port");
    let (global, handler) = mock_global_state(AppConfig {
        api: ApiConfig {
            bind_address: format!("0.0.0.0:{}", port).parse().unwrap(),
            tls: None,
        },
        ..Default::default()
    })
    .await;

    sqlx::query!("DELETE FROM users")
        .execute(global.db.as_ref())
        .await
        .expect("failed to clear users");
    let id = sqlx::query!(
        "INSERT INTO users (username, display_name, email, password_hash, stream_key) VALUES ($1, $1, $2, $3, $4) RETURNING id",
        "test",
        "test@test.com",
        user::hash_password("test"),
        user::generate_stream_key(),
    )
    .map(|row| row.id)
    .fetch_one(global.db.as_ref())
    .await
    .expect("failed to insert user");

    let session = sqlx::query_as!(
        session::Model,
        "INSERT INTO sessions (user_id, expires_at) VALUES ($1, $2) RETURNING *",
        id,
        Utc::now() - Duration::seconds(30)
    )
    .fetch_one(global.db.as_ref())
    .await
    .expect("failed to insert session");

    let token = JwtState::from(session)
        .serialize(&global)
        .expect("failed to create token");

    let handle = tokio::spawn(api::run(global));

    // We need to wait for the server to start
    tokio::time::sleep(time::Duration::from_millis(300)).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://localhost:{}/v1/health", port))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Auth-Token-Check", "always")
        .send()
        .await
        .expect("failed to get health");

    assert_eq!(resp.status(), http::StatusCode::UNAUTHORIZED);
    let body: Value = resp.json().await.expect("failed to read body");
    assert_eq!(
        body,
        json!({
            "message": "unauthorized",
            "success": false,
        })
    );

    // The client uses Keep-Alive, so we need to drop it to release the global context
    drop(client);

    handler
        .cancel()
        .timeout(time::Duration::from_secs(1))
        .await
        .expect("failed to cancel context");

    handle
        .timeout(time::Duration::from_secs(1))
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
