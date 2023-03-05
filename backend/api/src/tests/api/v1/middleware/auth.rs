use core::time;

use chrono::{Duration, Utc};
use common::types::session;
use http::header;
use serde_json::{json, Value};

use crate::{
    api::{self, v1::jwt::JwtState},
    config::AppConfig,
    tests::global::mock_global_state,
};

#[tokio::test]

async fn test_auth_middleware() {
    let (global, handler) = mock_global_state(AppConfig {
        bind_address: "0.0.0.0:8081".to_string(),
        ..Default::default()
    })
    .await;

    sqlx::query!("DELETE FROM users")
        .execute(&*global.db)
        .await
        .expect("failed to clear users");
    sqlx::query!(
        "INSERT INTO users (id, username, email, password_hash) VALUES ($1, $2, $3, $4)",
        1,
        "test",
        "test@test.com",
        "$2b$1"
    )
    .execute(&*global.db)
    .await
    .expect("failed to insert user");

    let session = sqlx::query_as!(
        session::Model,
        "INSERT INTO sessions (user_id, expires_at) VALUES ($1, $2) RETURNING *",
        1,
        Utc::now() + Duration::seconds(30)
    )
    .fetch_one(&*global.db)
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
        .get("http://localhost:8081/v1/health")
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
    .execute(&*global.db)
    .await
    .expect("failed to update session");

    let resp = client
        .get("http://localhost:8081/v1/health")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .await
        .expect("failed to get health");

    assert_eq!(resp.status(), http::StatusCode::UNAUTHORIZED);
    let body: Value = resp.json().await.expect("failed to read body");
    assert_eq!(
        body,
        json!({"success": false, "message": "session token has been invalidated"})
    );

    // The client uses Keep-Alive, so we need to drop it to release the global context
    drop(global);
    drop(client);

    tokio::time::timeout(time::Duration::from_secs(1), handler.cancel())
        .await
        .expect("failed to cancel context");

    tokio::time::timeout(time::Duration::from_secs(1), handle)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn test_auth_middleware_failed() {
    let (global, handler) = mock_global_state(AppConfig {
        bind_address: "0.0.0.0:8081".to_string(),
        ..Default::default()
    })
    .await;

    sqlx::query!("DELETE FROM users")
        .execute(&*global.db)
        .await
        .expect("failed to clear users");
    sqlx::query!(
        "INSERT INTO users (id, username, email, password_hash) VALUES ($1, $2, $3, $4)",
        1,
        "test",
        "test@test.com",
        "$2b$1"
    )
    .execute(&*global.db)
    .await
    .expect("failed to insert user");

    let session = sqlx::query_as!(
        session::Model,
        "INSERT INTO sessions (user_id, expires_at) VALUES ($1, $2) RETURNING *",
        1,
        Utc::now() - Duration::seconds(30)
    )
    .fetch_one(&*global.db)
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
        .get("http://localhost:8081/v1/health")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .await
        .expect("failed to get health");

    assert_eq!(resp.status(), http::StatusCode::UNAUTHORIZED);
    let body: Value = resp.json().await.expect("failed to read body");
    assert_eq!(
        body,
        json!({
            "message": "invalid authentication token",
            "success": false,
        })
    );

    // The client uses Keep-Alive, so we need to drop it to release the global context
    drop(client);

    tokio::time::timeout(time::Duration::from_secs(1), handler.cancel())
        .await
        .expect("failed to cancel context");

    tokio::time::timeout(time::Duration::from_secs(1), handle)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
