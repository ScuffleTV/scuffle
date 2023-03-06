use std::{sync::Arc, time::Duration};

use arc_swap::ArcSwap;
use async_graphql::{Name, Request, Variables};
use chrono::Utc;
use common::types::{session, user};
use serde_json::json;

use crate::{
    api::v1::{
        gql::{schema, GqlContext},
        jwt::JwtState,
    },
    config::AppConfig,
    tests::global::{mock_global_state, turnstile::mock_turnstile},
};

#[tokio::test]
async fn test_login() {
    let (mut rx, addr, h1) = mock_turnstile().await;
    let (global, handler) = mock_global_state(AppConfig {
        turnstile_url: addr,
        turnstile_secret_key: "DUMMY_KEY__DEADBEEF".to_string(),
        ..Default::default()
    })
    .await;

    sqlx::query!("DELETE FROM users")
        .execute(&*global.db)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO users(id, username, email, password_hash) VALUES ($1, $2, $3, $4)",
        1,
        "admin",
        "admin@admin.com",
        user::hash_password("admin")
    )
    .execute(&*global.db)
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

    let ctx = Arc::new(GqlContext::default());
    let res = tokio::time::timeout(
        Duration::from_secs(1),
        schema.execute(Request::from(query).data(global.clone()).data(ctx)),
    )
    .await
    .unwrap();

    assert_eq!(res.errors.len(), 0);
    let json = res.data.into_json();
    assert!(json.is_ok());

    let session = tokio::time::timeout(
        Duration::from_secs(1),
        sqlx::query_as!(session::Model, "SELECT * FROM sessions").fetch_one(&*global.db),
    )
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

    tokio::time::timeout(Duration::from_secs(1), h1)
        .await
        .unwrap()
        .ok(); // ignore error because we aborted it
    tokio::time::timeout(Duration::from_secs(1), h2)
        .await
        .unwrap()
        .unwrap();

    drop(global);

    tokio::time::timeout(Duration::from_secs(1), handler.cancel())
        .await
        .expect("failed to cancel context");
}

#[tokio::test]
async fn test_login_while_logged_in() {
    let (global, handler) = mock_global_state(Default::default()).await;

    sqlx::query!("DELETE FROM users")
        .execute(&*global.db)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO users(id, username, email, password_hash) VALUES ($1, $2, $3, $4)",
        1,
        "admin",
        "admin@admin.com",
        user::hash_password("admin")
    )
    .execute(&*global.db)
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

    let ctx = Arc::new(GqlContext {
        is_websocket: true,
        session: ArcSwap::from_pointee(Some(Default::default())),
    });
    let res = tokio::time::timeout(
        Duration::from_secs(1),
        schema.execute(Request::from(query).data(global.clone()).data(ctx)),
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
    assert_eq!(errors, vec![json!("Already logged in")]);

    drop(global);

    tokio::time::timeout(Duration::from_secs(1), handler.cancel())
        .await
        .expect("failed to cancel context");
}

#[tokio::test]
async fn test_login_with_token() {
    let (global, handler) = mock_global_state(Default::default()).await;

    sqlx::query!("DELETE FROM users")
        .execute(&*global.db)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO users(id, username, email, password_hash) VALUES ($1, $2, $3, $4)",
        1,
        "admin",
        "admin@admin.com",
        user::hash_password("admin")
    )
    .execute(&*global.db)
    .await
    .unwrap();
    let session = sqlx::query_as!(
        session::Model,
        "INSERT INTO sessions(id, user_id, expires_at) VALUES ($1, $2, $3) RETURNING *",
        1,
        1,
        Utc::now() + chrono::Duration::seconds(60)
    )
    .fetch_one(&*global.db)
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

    let ctx = Arc::new(GqlContext::default());
    let res = tokio::time::timeout(
        Duration::from_secs(1),
        schema.execute(
            Request::from(query)
                .variables(variables)
                .data(global.clone())
                .data(ctx),
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

    tokio::time::timeout(Duration::from_secs(1), handler.cancel())
        .await
        .expect("failed to cancel context");
}

#[tokio::test]
async fn test_login_with_session_expired() {
    let (global, handler) = mock_global_state(Default::default()).await;

    sqlx::query!("DELETE FROM users")
        .execute(&*global.db)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO users(id, username, email, password_hash) VALUES ($1, $2, $3, $4)",
        1,
        "admin",
        "admin@admin.com",
        user::hash_password("admin")
    )
    .execute(&*global.db)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO sessions(id, user_id, expires_at) VALUES ($1, $2, $3)",
        1,
        1,
        Utc::now() - chrono::Duration::seconds(60)
    )
    .execute(&*global.db)
    .await
    .unwrap();

    let schema = schema();
    let query = r#"
        mutation {
            auth {
                loginWithToken(sessionToken: "token") {
                    token
                }
            }
        }
    "#;

    let ctx = Arc::new(GqlContext::default());
    let res = tokio::time::timeout(
        Duration::from_secs(1),
        schema.execute(Request::from(query).data(global.clone()).data(ctx)),
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

    tokio::time::timeout(Duration::from_secs(1), handler.cancel())
        .await
        .expect("failed to cancel context");
}

#[tokio::test]
async fn test_login_while_logged_in_with_session_expired() {
    let (global, handler) = mock_global_state(Default::default()).await;

    sqlx::query!("DELETE FROM users")
        .execute(&*global.db)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO users(id, username, email, password_hash) VALUES ($1, $2, $3, $4)",
        1,
        "admin",
        "admin@admin.com",
        user::hash_password("admin")
    )
    .execute(&*global.db)
    .await
    .unwrap();
    let session = sqlx::query_as!(
        session::Model,
        "INSERT INTO sessions(id, user_id, expires_at) VALUES ($1, $2, $3) RETURNING *",
        1,
        1,
        Utc::now() - chrono::Duration::seconds(60)
    )
    .fetch_one(&*global.db)
    .await
    .unwrap();

    let schema = schema();
    let query = r#"
        mutation {
            auth {
                loginWithToken(sessionToken: "token") {
                    token
                }
            }
        }
    "#;

    let ctx = Arc::new(GqlContext {
        is_websocket: true,
        session: ArcSwap::from_pointee(Some(session)),
    });
    let res = tokio::time::timeout(
        Duration::from_secs(1),
        schema.execute(Request::from(query).data(global.clone()).data(ctx)),
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
    assert_eq!(errors, vec![json!("Session is no longer valid")]);

    drop(global);

    tokio::time::timeout(Duration::from_secs(1), handler.cancel())
        .await
        .expect("failed to cancel context");
}

#[tokio::test]
async fn test_register() {
    let (mut rx, addr, h1) = mock_turnstile().await;
    let (global, handler) = mock_global_state(AppConfig {
        turnstile_url: addr,
        turnstile_secret_key: "DUMMY_KEY__LOREM_IPSUM".to_string(),
        ..Default::default()
    })
    .await;

    sqlx::query!("DELETE FROM users")
        .execute(&*global.db)
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

    let ctx = Arc::new(GqlContext::default());
    let res = tokio::time::timeout(
        Duration::from_secs(1),
        schema.execute(Request::from(query).data(global.clone()).data(ctx)),
    )
    .await
    .unwrap();

    assert_eq!(res.errors.len(), 0);
    let json = res.data.into_json();
    assert!(json.is_ok());

    let user = tokio::time::timeout(
        Duration::from_secs(1),
        sqlx::query_as!(user::Model, "SELECT * FROM users").fetch_one(&*global.db),
    )
    .await
    .unwrap()
    .unwrap();

    assert_eq!(user.username, "admin");
    assert_eq!(user.email, "admin@admin.com");
    assert!(user.verify_password("SuperStr0ngP@ssword!"));

    let session = tokio::time::timeout(
        Duration::from_secs(1),
        sqlx::query_as!(session::Model, "SELECT * FROM sessions").fetch_one(&*global.db),
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

    tokio::time::timeout(Duration::from_secs(1), h1)
        .await
        .unwrap()
        .ok(); // ignore error because we aborted it
    tokio::time::timeout(Duration::from_secs(1), h2)
        .await
        .unwrap()
        .unwrap();

    drop(global);

    tokio::time::timeout(Duration::from_secs(1), handler.cancel())
        .await
        .expect("failed to cancel context");
}

#[tokio::test]
async fn test_logout() {
    let (global, handler) = mock_global_state(Default::default()).await;

    sqlx::query!("DELETE FROM users")
        .execute(&*global.db)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO users(id, username, email, password_hash) VALUES ($1, $2, $3, $4)",
        1,
        "admin",
        "admin@admin.com",
        user::hash_password("admin")
    )
    .execute(&*global.db)
    .await
    .unwrap();
    let session = sqlx::query_as!(
        session::Model,
        "INSERT INTO sessions(id, user_id, expires_at) VALUES ($1, $2, $3) RETURNING *",
        1,
        1,
        Utc::now() + chrono::Duration::seconds(60)
    )
    .fetch_one(&*global.db)
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

    let ctx = Arc::new(GqlContext {
        is_websocket: false,
        session: ArcSwap::from_pointee(Some(session)),
    });

    let res = tokio::time::timeout(
        Duration::from_secs(1),
        schema.execute(Request::from(query).data(global.clone()).data(ctx)),
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
        sqlx::query_as!(session::Model, "SELECT * FROM sessions").fetch_one(&*global.db),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(!session.is_valid());

    drop(global);

    tokio::time::timeout(Duration::from_secs(1), handler.cancel())
        .await
        .expect("failed to cancel context");
}

#[tokio::test]
async fn test_logout_with_token() {
    let (global, handler) = mock_global_state(Default::default()).await;

    sqlx::query!("DELETE FROM users")
        .execute(&*global.db)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO users(id, username, email, password_hash) VALUES ($1, $2, $3, $4)",
        1,
        "admin",
        "admin@admin.com",
        user::hash_password("admin")
    )
    .execute(&*global.db)
    .await
    .unwrap();
    let session = sqlx::query_as!(
        session::Model,
        "INSERT INTO sessions(id, user_id, expires_at) VALUES ($1, $2, $3) RETURNING *",
        1,
        1,
        Utc::now() + chrono::Duration::seconds(60)
    )
    .fetch_one(&*global.db)
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

    let ctx = Arc::new(GqlContext::default());

    let mut variables = Variables::default();
    variables.insert(Name::new("token"), async_graphql::Value::String(token));

    let res = tokio::time::timeout(
        Duration::from_secs(1),
        schema.execute(
            Request::from(query)
                .variables(variables)
                .data(global.clone())
                .data(ctx),
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
        sqlx::query_as!(session::Model, "SELECT * FROM sessions").fetch_one(&*global.db),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(!session.is_valid());

    drop(global);

    tokio::time::timeout(Duration::from_secs(1), handler.cancel())
        .await
        .expect("failed to cancel context");
}
