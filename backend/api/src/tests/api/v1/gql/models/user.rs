use std::sync::Arc;
use std::time::Duration;

use crate::api::v1::gql::{ext::RequestExt, request_context::RequestContext, schema};
use crate::tests::global::mock_global_state;
use async_graphql::{Request, Value};
use common::types::{session, user};

#[tokio::test]
async fn test_user_by_name() {
    let (global, handler) = mock_global_state(Default::default()).await;

    sqlx::query!("DELETE FROM users")
        .execute(&*global.db)
        .await
        .unwrap();
    let user =
        sqlx::query_as!(user::Model,
        "INSERT INTO users(id, username, email, password_hash) VALUES ($1, $2, $3, $4) RETURNING *",
        1,
        "admin",
        "admin@admin.com",
        user::hash_password("admin")
    )
        .fetch_one(&*global.db)
        .await
        .unwrap();

    let session = sqlx::query_as!(
        session::Model,
        "INSERT INTO sessions(user_id, expires_at) VALUES ($1, $2) RETURNING *",
        1,
        chrono::Utc::now() + chrono::Duration::seconds(30)
    )
    .fetch_one(&*global.db)
    .await
    .unwrap();

    let schema = schema();

    {
        let query = r#"
            query {
                userByUsername(username: "admin") {
                    id
                    username
                    createdAt
                }
            }
        "#;

        let ctx = Arc::new(RequestContext::new(false));

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

        assert!(res.is_ok());
        assert_eq!(res.errors.len(), 0);
        let json = res.data.into_json();
        assert!(json.is_ok());

        assert_eq!(
            json.unwrap(),
            serde_json::json!({ "userByUsername": { "id": user.id, "username": user.username, "createdAt": user.created_at.to_rfc3339() } })
        );
    }
    {
        let query = r#"
            query {
                userByUsername(username: "admin") {
                    id
                    email
                    username
                    createdAt
                }
            }
        "#;

        let ctx = Arc::new(RequestContext::new(false));

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

        assert!(!res.is_ok());
        assert_eq!(res.errors.len(), 1);
        let json = res.data.into_json();
        assert!(json.is_ok());

        assert_eq!(json.unwrap(), serde_json::json!({ "userByUsername": null }));

        assert_eq!(
            res.errors[0].message,
            "Unauthorized: you are not allowed to see this field"
        );

        let extensions = res.errors[0].extensions.as_ref().unwrap();

        assert_eq!(extensions.get("fields"), Some(&Value::from(vec!["email"])));

        assert_eq!(extensions.get("kind"), Some(&Value::from("Unauthorized")));
    }
    {
        let query = r#"
            query {
                userByUsername(username: "admin") {
                    id
                    emailVerified
                    username
                    createdAt
                }
            }
        "#;

        let ctx = Arc::new(RequestContext::new(false));

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

        assert!(!res.is_ok());
        assert_eq!(res.errors.len(), 1);
        let json = res.data.into_json();
        assert!(json.is_ok());

        assert_eq!(json.unwrap(), serde_json::json!({ "userByUsername": null }));

        assert_eq!(
            res.errors[0].message,
            "Unauthorized: you are not allowed to see this field"
        );

        let extensions = res.errors[0].extensions.as_ref().unwrap();

        assert_eq!(
            extensions.get("fields"),
            Some(&Value::from(vec!["emailVerified"]))
        );

        assert_eq!(extensions.get("kind"), Some(&Value::from("Unauthorized")));
    }
    {
        let query = r#"
            query {
                userByUsername(username: "admin") {
                    id
                    lastLoginAt
                    username
                    createdAt
                }
            }
        "#;

        let ctx = Arc::new(RequestContext::new(false));

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

        assert!(!res.is_ok());
        assert_eq!(res.errors.len(), 1);
        let json = res.data.into_json();
        assert!(json.is_ok());

        assert_eq!(json.unwrap(), serde_json::json!({ "userByUsername": null }));

        assert_eq!(
            res.errors[0].message,
            "Unauthorized: you are not allowed to see this field"
        );

        let extensions = res.errors[0].extensions.as_ref().unwrap();

        assert_eq!(
            extensions.get("fields"),
            Some(&Value::from(vec!["lastLoginAt"]))
        );

        assert_eq!(extensions.get("kind"), Some(&Value::from("Unauthorized")));
    }
    {
        let query = r#"
            query {
                userByUsername(username: "admin") {
                    id
                    email
                    emailVerified
                    lastLoginAt
                    username
                    createdAt
                }
            }
        "#;

        let ctx = Arc::new(RequestContext::new(false));
        ctx.set_session(Some(session.clone()));

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

        assert!(res.is_ok());
        assert_eq!(res.errors.len(), 0);
        let json = res.data.into_json();
        assert!(json.is_ok());

        assert_eq!(
            json.unwrap(),
            serde_json::json!({ "userByUsername": { "id": user.id, "email": user.email, "emailVerified": user.email_verified, "lastLoginAt": user.last_login_at.to_rfc3339(), "username": user.username, "createdAt": user.created_at.to_rfc3339() } })
        );
    }

    sqlx::query!("DELETE FROM sessions WHERE id = $1", session.id)
        .execute(&*global.db)
        .await
        .expect("failed to delete user");

    {
        let query = r#"
            query {
                userByUsername(username: "admin") {
                    id
                    lastLoginAt
                    username
                    createdAt
                }
            }
        "#;

        let ctx = Arc::new(RequestContext::new(true));
        ctx.set_session(Some(session.clone()));

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

        assert!(!res.is_ok());
        assert_eq!(res.errors.len(), 1);
        let json = res.data.into_json();
        assert!(json.is_ok());

        assert_eq!(json.unwrap(), serde_json::json!({ "userByUsername": null }));

        assert_eq!(
            res.errors[0].message,
            "InvalidSession: Session is no longer valid"
        );

        let extensions = res.errors[0].extensions.as_ref().unwrap();

        assert_eq!(extensions.get("kind"), Some(&Value::from("InvalidSession")));
    }

    drop(global);

    tokio::time::timeout(Duration::from_secs(1), handler.cancel())
        .await
        .expect("failed to cancel context");
}
