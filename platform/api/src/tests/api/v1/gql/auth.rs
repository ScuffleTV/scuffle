use std::{sync::Arc, time::Duration};

use crate::database::{session, user};
use async_graphql::{Name, Request, Variables};
use chrono::Utc;
use common::prelude::FutureTimeout;
use serde_json::json;
use serial_test::serial;

use crate::{
    api::v1::{
        gql::{ext::RequestExt, request_context::RequestContext, schema},
        jwt::JwtState,
    },
    config::{AppConfig, TurnstileConfig},
    tests::global::{mock_global_state, turnstile::mock_turnstile},
};

#[serial]
#[tokio::test]
async fn test_serial_login() {
    let (mut rx, addr, h1) = mock_turnstile().await;
    let (global, handler) = mock_global_state(AppConfig {
        turnstile: TurnstileConfig {
            url: addr,
            secret_key: "DUMMY_KEY__DEADBEEF".to_string(),
        },
        ..Default::default()
    })
    .await;

    sqlx::query!("DELETE FROM users")
        .execute(global.db.as_ref())
        .await
        .unwrap();
    sqlx::query_as!(user::Model,
        "INSERT INTO users(username, display_name, email, password_hash, stream_key) VALUES ($1, $1, $2, $3, $4) RETURNING *",
        "admin",
        "admin@admin.com",
        user::hash_password("admin"),
        user::generate_stream_key(),
    )
    .fetch_one(global.db.as_ref())
    .await
    .unwrap();

    let schema = schema();
    let query = r#"
        mutation {
            auth {
                login(username: "admin", password: "admin", captchaToken: "1234") {
                    token
                }
            }
        }
    "#;

    let h2 = tokio::spawn(async move {
        let (req, resp) = rx.recv().await.unwrap();
        assert_eq!(req.response, "1234");
        assert_eq!(req.secret, "DUMMY_KEY__DEADBEEF");

        resp.send(true).unwrap();
    });

    let ctx = Arc::new(RequestContext::new(false));
    let res = schema
        .execute(
            Request::from(query)
                .provide_global(global.clone())
                .provide_context(ctx),
        )
        .timeout(Duration::from_secs(5))
        .await
        .unwrap();

    assert_eq!(res.errors.len(), 0);
    let json = res.data.into_json();
    assert!(json.is_ok());

    let session = sqlx::query_as!(session::Model, "SELECT * FROM sessions")
        .fetch_one(global.db.as_ref())
        .timeout(Duration::from_secs(5))
        .await
        .unwrap()
        .unwrap();

    let jwt_state = JwtState::from(session);

    let token = jwt_state
        .serialize(&global)
        .expect("failed to serialize jwt state");

    assert_eq!(
        json.unwrap(),
        serde_json::json!({ "auth": { "login": { "token": token }} })
    );

    h1.abort();

    h1.timeout(Duration::from_secs(1)).await.unwrap().ok(); // ignore error because we aborted it
    h2.timeout(Duration::from_secs(1)).await.unwrap().unwrap();

    drop(global);

    handler
        .cancel()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel context");
}

#[serial]
#[tokio::test]
async fn test_serial_login_while_logged_in() {
    let (mut rx, addr, h1) = mock_turnstile().await;
    let (global, handler) = mock_global_state(AppConfig {
        turnstile: TurnstileConfig {
            url: addr,
            secret_key: "batman's chest".to_string(),
        },
        ..Default::default()
    })
    .await;

    sqlx::query!("DELETE FROM users")
        .execute(global.db.as_ref())
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO users(username, display_name, email, password_hash, stream_key) VALUES ($1, $1, $2, $3, $4)",
        "admin",
        "admin@admin.com",
        user::hash_password("admin"),
        user::generate_stream_key(),
    )
    .execute(global.db.as_ref())
    .await
    .unwrap();

    let schema = schema();
    let query = r#"
        mutation {
            auth {
                login(username: "admin", password: "admin", captchaToken: "1234") {
                    token
                }
            }
        }
    "#;

    let ctx = Arc::new(RequestContext::new(true));
    ctx.set_session(Some(Default::default()));

    let h2 = tokio::spawn(async move {
        let (req, resp) = rx.recv().await.unwrap();
        assert_eq!(req.response, "1234");
        assert_eq!(req.secret, "batman's chest");

        resp.send(true).unwrap();
    });

    let res = schema
        .execute(
            Request::from(query)
                .provide_context(ctx)
                .provide_global(global.clone()),
        )
        .timeout(Duration::from_secs(2))
        .await
        .unwrap();

    assert_eq!(res.errors.len(), 0);
    let json = res.data.into_json();
    assert!(json.is_ok());

    let session = sqlx::query_as!(session::Model, "SELECT * FROM sessions")
        .fetch_one(global.db.as_ref())
        .timeout(Duration::from_secs(1))
        .await
        .unwrap()
        .unwrap();

    let jwt_state = JwtState::from(session);

    let token = jwt_state
        .serialize(&global)
        .expect("failed to serialize jwt state");

    assert_eq!(
        json.unwrap(),
        serde_json::json!({ "auth": { "login": { "token": token }} })
    );

    h1.abort();

    h1.timeout(Duration::from_secs(1)).await.unwrap().ok(); // ignore error because we aborted it

    h2.timeout(Duration::from_secs(1)).await.unwrap().unwrap();

    drop(global);

    handler
        .cancel()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel context");
}

#[serial]
#[tokio::test]
async fn test_serial_login_with_token() {
    let (global, handler) = mock_global_state(Default::default()).await;

    sqlx::query!("DELETE FROM users")
        .execute(global.db.as_ref())
        .await
        .unwrap();
    let user = sqlx::query!(
        "INSERT INTO users(username, display_name, email, password_hash, stream_key) VALUES ($1, $1, $2, $3, $4) RETURNING *",
        "admin",
        "admin@admin.com",
        user::hash_password("admin"),
        user::generate_stream_key()
    )
    .fetch_one(global.db.as_ref())
    .await
    .unwrap();

    let session = sqlx::query_as!(
        session::Model,
        "INSERT INTO sessions(user_id, expires_at) VALUES ($1, $2) RETURNING *",
        user.id,
        Utc::now() + chrono::Duration::seconds(60)
    )
    .fetch_one(global.db.as_ref())
    .await
    .unwrap();
    let token = JwtState::from(session).serialize(&global).unwrap();

    let schema = schema();
    let query = r#"
        mutation($token: String!) {
            auth {
                loginWithToken(sessionToken: $token) {
                    token
                }
            }
        }
    "#;

    let mut variables = Variables::default();
    variables.insert(
        Name::new("token"),
        async_graphql::Value::String(token.clone()),
    );

    let ctx = Arc::new(RequestContext::new(false));
    let res = tokio::time::timeout(
        Duration::from_secs(1),
        schema.execute(
            Request::from(query)
                .variables(variables)
                .provide_context(ctx.clone())
                .provide_global(global.clone()),
        ),
    )
    .await
    .unwrap();

    assert_eq!(res.errors.len(), 0);
    let json = res.data.into_json();
    assert!(json.is_ok());

    assert_eq!(
        json.unwrap(),
        serde_json::json!({ "auth": { "loginWithToken": { "token": token }} })
    );

    drop(global);

    handler
        .cancel()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel context");
}

#[serial]
#[tokio::test]
async fn test_serial_login_with_session_expired() {
    let (global, handler) = mock_global_state(Default::default()).await;

    sqlx::query!("DELETE FROM users")
        .execute(global.db.as_ref())
        .await
        .unwrap();
    let user = sqlx::query!(
        "INSERT INTO users(username, display_name, email, password_hash, stream_key) VALUES ($1, $1, $2, $3, $4) RETURNING *",
        "admin",
        "admin@admin.com",
        user::hash_password("admin"),
        user::generate_stream_key()
    )
    .fetch_one(global.db.as_ref())
    .await
    .unwrap();
    let session = sqlx::query_as!(
        session::Model,
        "INSERT INTO sessions(user_id, expires_at) VALUES ($1, $2) RETURNING *",
        user.id,
        Utc::now() - chrono::Duration::seconds(60)
    )
    .fetch_one(global.db.as_ref())
    .await
    .unwrap();

    let schema = schema();
    let query = r#"
        mutation Login($token: String!) {
            auth {
                loginWithToken(sessionToken: $token) {
                    token
                }
            }
        }
    "#;

    let jwt_state = JwtState::from(session);

    let mut variables = Variables::default();
    variables.insert(
        Name::new("token"),
        async_graphql::Value::String(jwt_state.serialize(&global).unwrap()),
    );

    let ctx = Arc::new(RequestContext::new(false));
    let res = tokio::time::timeout(
        Duration::from_secs(1),
        schema.execute(
            Request::from(query)
                .variables(variables)
                .provide_global(global.clone())
                .provide_context(ctx.clone()),
        ),
    )
    .await
    .unwrap();

    assert_eq!(res.errors.len(), 1);
    let json = res.data.into_json();
    assert!(json.is_ok());

    assert_eq!(json.unwrap(), serde_json::json!(null));

    let errors = res
        .errors
        .into_iter()
        .map(|e| {
            e.extensions
                .unwrap()
                .get("reason")
                .unwrap()
                .clone()
                .into_json()
                .unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(errors, vec![json!("Invalid session token")]);

    drop(global);

    handler
        .cancel()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel context");
}

#[serial]
#[tokio::test]
async fn test_serial_login_while_logged_in_with_session_expired() {
    let (global, handler) = mock_global_state(Default::default()).await;

    sqlx::query!("DELETE FROM users")
        .execute(global.db.as_ref())
        .await
        .unwrap();
    let user = sqlx::query_as!(user::Model,
        "INSERT INTO users(username, display_name, email, password_hash, stream_key) VALUES ($1, $1, $2, $3, $4) RETURNING *",
        "admin",
        "admin@admin.com",
        user::hash_password("admin"),
        user::generate_stream_key()
    )
    .fetch_one(global.db.as_ref())
    .await
    .unwrap();
    let session = sqlx::query_as!(
        session::Model,
        "INSERT INTO sessions(user_id, expires_at) VALUES ($1, $2) RETURNING *",
        user.id,
        Utc::now() - chrono::Duration::seconds(60)
    )
    .fetch_one(global.db.as_ref())
    .await
    .unwrap();

    let session2 = sqlx::query_as!(
        session::Model,
        "INSERT INTO sessions(user_id, expires_at) VALUES ($1, $2) RETURNING *",
        user.id,
        Utc::now() + chrono::Duration::seconds(60)
    )
    .fetch_one(global.db.as_ref())
    .await
    .unwrap();

    let schema = schema();
    let query = r#"
        mutation Login($token: String!) {
            auth {
                loginWithToken(sessionToken: $token) {
                    token
                }
            }
        }
    "#;

    let jwt_state = JwtState::from(session2);

    let mut variables = Variables::default();
    variables.insert(
        Name::new("token"),
        async_graphql::Value::String(jwt_state.serialize(&global).unwrap()),
    );

    let ctx = Arc::new(RequestContext::new(true));
    ctx.set_session(Some((session, Default::default())));

    let res = tokio::time::timeout(
        Duration::from_secs(1),
        schema.execute(
            Request::from(query)
                .variables(variables)
                .provide_global(global.clone())
                .provide_context(ctx.clone()),
        ),
    )
    .await
    .unwrap();

    println!("{:?}", res.errors);

    assert_eq!(res.errors.len(), 0);
    let json = res.data.into_json();
    assert!(json.is_ok());

    assert_eq!(
        json.unwrap(),
        serde_json::json!({ "auth": { "loginWithToken": { "token": jwt_state.serialize(&global).unwrap() }}})
    );
    drop(global);

    handler
        .cancel()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel context");
}

#[serial]
#[tokio::test]
async fn test_serial_register() {
    let (mut rx, addr, h1) = mock_turnstile().await;
    let (global, handler) = mock_global_state(AppConfig {
        turnstile: TurnstileConfig {
            url: addr,
            secret_key: "DUMMY_KEY__LOREM_IPSUM".to_string(),
        },
        ..Default::default()
    })
    .await;

    sqlx::query!("DELETE FROM users")
        .execute(global.db.as_ref())
        .await
        .unwrap();

    let schema = schema();
    let query = r#"
        mutation {
            auth {
                register(username: "admin", password: "SuperStr0ngP@ssword!", email: "admin@admin.com", captchaToken: "1234") {
                    token
                }
            }
        }
    "#;

    let h2 = tokio::spawn(async move {
        let (req, resp) = rx.recv().await.unwrap();
        assert_eq!(req.response, "1234");
        assert_eq!(req.secret, "DUMMY_KEY__LOREM_IPSUM");

        resp.send(true).unwrap();
    });

    let ctx = Arc::new(RequestContext::new(false));
    let res = tokio::time::timeout(
        Duration::from_secs(2),
        schema.execute(
            Request::from(query)
                .provide_global(global.clone())
                .provide_context(ctx),
        ),
    )
    .await
    .unwrap();

    assert_eq!(res.errors.len(), 0);
    let json = res.data.into_json();
    assert!(json.is_ok());

    let user = tokio::time::timeout(
        Duration::from_secs(1),
        sqlx::query_as!(user::Model, "SELECT * FROM users").fetch_one(global.db.as_ref()),
    )
    .await
    .unwrap()
    .unwrap();

    assert_eq!(user.username, "admin");
    assert_eq!(user.email, "admin@admin.com");
    assert!(user.verify_password("SuperStr0ngP@ssword!"));

    let session = tokio::time::timeout(
        Duration::from_secs(1),
        sqlx::query_as!(session::Model, "SELECT * FROM sessions").fetch_one(global.db.as_ref()),
    )
    .await
    .unwrap()
    .unwrap();
    let token = JwtState::from(session).serialize(&global).unwrap();
    assert_eq!(
        json.unwrap(),
        serde_json::json!({ "auth": { "register": { "token": token }} })
    );

    h1.abort();

    h1.timeout(Duration::from_secs(1)).await.unwrap().ok(); // ignore error because we aborted it
    h2.timeout(Duration::from_secs(1)).await.unwrap().unwrap();

    drop(global);

    handler
        .cancel()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel context");
}

#[serial]
#[tokio::test]
async fn test_serial_logout() {
    let (global, handler) = mock_global_state(Default::default()).await;

    sqlx::query!("DELETE FROM users")
        .execute(global.db.as_ref())
        .await
        .unwrap();
    let user = sqlx::query_as!(user::Model,
        "INSERT INTO users(username, display_name, email, password_hash, stream_key) VALUES ($1, $1, $2, $3, $4) RETURNING *",
        "admin",
        "admin@admin.com",
        user::hash_password("admin"),
        user::generate_stream_key(),
    )
    .fetch_one(global.db.as_ref())
    .await
    .unwrap();
    let session = sqlx::query_as!(
        session::Model,
        "INSERT INTO sessions(user_id, expires_at) VALUES ($1, $2) RETURNING *",
        user.id,
        Utc::now() + chrono::Duration::seconds(60)
    )
    .fetch_one(global.db.as_ref())
    .await
    .unwrap();

    let schema = schema();
    let query = r#"
        mutation {
            auth {
                logout
            }
        }
    "#;

    let ctx = Arc::new(RequestContext::new(false));
    ctx.set_session(Some((session, Default::default())));

    let res = tokio::time::timeout(
        Duration::from_secs(1),
        schema.execute(
            Request::from(query)
                .provide_global(global.clone())
                .provide_context(ctx),
        ),
    )
    .await
    .unwrap();

    assert_eq!(res.errors.len(), 0);
    let json = res.data.into_json();
    assert!(json.is_ok());

    assert_eq!(
        json.unwrap(),
        serde_json::json!({ "auth": { "logout": true }})
    );

    let session = tokio::time::timeout(
        Duration::from_secs(1),
        sqlx::query_as!(session::Model, "SELECT * FROM sessions").fetch_one(global.db.as_ref()),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(!session.is_valid());

    drop(global);

    handler
        .cancel()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel context");
}

#[serial]
#[tokio::test]
async fn test_serial_logout_with_token() {
    let (global, handler) = mock_global_state(Default::default()).await;

    sqlx::query!("DELETE FROM users")
        .execute(global.db.as_ref())
        .await
        .unwrap();
    let user = sqlx::query_as!(user::Model,
        "INSERT INTO users(username, display_name, email, password_hash, stream_key) VALUES ($1, $1, $2, $3, $4) RETURNING *",
        "admin",
        "admin@admin.com",
        user::hash_password("admin"),
        user::generate_stream_key()
    )
    .fetch_one(global.db.as_ref())
    .await
    .unwrap();
    let session = sqlx::query_as!(
        session::Model,
        "INSERT INTO sessions(user_id, expires_at) VALUES ($1, $2) RETURNING *",
        user.id,
        Utc::now() + chrono::Duration::seconds(60)
    )
    .fetch_one(global.db.as_ref())
    .await
    .unwrap();
    let token = JwtState::from(session.clone()).serialize(&global).unwrap();

    let schema = schema();
    let query = r#"
        mutation($token: String!) {
            auth {
                logout(sessionToken: $token)
            }
        }
    "#;

    let ctx = Arc::new(RequestContext::new(false));

    let mut variables = Variables::default();
    variables.insert(Name::new("token"), async_graphql::Value::String(token));

    let res = tokio::time::timeout(
        Duration::from_secs(1),
        schema.execute(
            Request::from(query)
                .variables(variables)
                .provide_global(global.clone())
                .provide_context(ctx.clone()),
        ),
    )
    .await
    .unwrap();

    assert_eq!(res.errors.len(), 0);
    let json = res.data.into_json();
    assert!(json.is_ok());

    assert_eq!(
        json.unwrap(),
        serde_json::json!({ "auth": { "logout": true }})
    );

    let session = tokio::time::timeout(
        Duration::from_secs(1),
        sqlx::query_as!(session::Model, "SELECT * FROM sessions").fetch_one(global.db.as_ref()),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(!session.is_valid());

    drop(global);

    handler
        .cancel()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel context");
}
