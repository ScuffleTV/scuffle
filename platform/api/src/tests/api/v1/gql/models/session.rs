use crate::api::v1::gql::{ext::RequestExt, request_context::RequestContext, schema};
use crate::api::v1::jwt::JwtState;
use crate::database::{session, user};
use crate::tests::global::mock_global_state;
use async_graphql::{Request, Variables};
use common::prelude::FutureTimeout;
use serial_test::serial;
use std::sync::Arc;
use std::time::Duration;

#[serial]
#[tokio::test]
async fn test_serial_session_user() {
    let (global, handler) = mock_global_state(Default::default()).await;

    sqlx::query!("DELETE FROM users")
        .execute(&*global.db)
        .await
        .unwrap();
    let user =
        sqlx::query_as!(user::Model,
        "INSERT INTO users(username, display_name, email, password_hash, stream_key) VALUES ($1, $1, $2, $3, $4) RETURNING *",
        "admin",
        "admin@admin.com",
        user::hash_password("admin"),
        user::generate_stream_key(),
    )
        .fetch_one(&*global.db)
        .await
        .unwrap();

    let session = sqlx::query_as!(
        session::Model,
        "INSERT INTO sessions(user_id, expires_at) VALUES ($1, $2) RETURNING *",
        user.id,
        chrono::Utc::now() + chrono::Duration::seconds(30)
    )
    .fetch_one(&*global.db)
    .await
    .unwrap();

    let schema = schema();

    {
        let query = r#"
            mutation TestLoginWithToken($token: String!) {
                auth {
                    loginWithToken(sessionToken: $token) {
                        id
                        user {
                            id
                            username
                            email
                        }
                    }
                }
            }
        "#;

        let ctx = Arc::new(RequestContext::new(false));

        let token = JwtState::from(session.clone()).serialize(&global).unwrap();

        let variables = Variables::from_json(serde_json::json!({ "token": token }));

        let res = tokio::time::timeout(
            Duration::from_secs(1),
            schema.execute(
                Request::from(query)
                    .variables(variables)
                    .provide_global(global.clone())
                    .provide_context(ctx),
            ),
        )
        .await
        .unwrap();

        println!("{:?}", res.errors);

        assert!(res.is_ok());
        assert_eq!(res.errors.len(), 0);
        let json = res.data.into_json();
        assert!(json.is_ok());

        assert_eq!(
            json.unwrap(),
            serde_json::json!({ "auth": { "loginWithToken": { "id": session.id, "user": { "id": user.id, "username": user.username, "email": user.email } } }})
        );
    }

    drop(global);

    handler
        .cancel()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel context");
}
