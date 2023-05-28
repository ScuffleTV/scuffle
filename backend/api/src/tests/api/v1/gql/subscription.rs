use std::{sync::Arc, time::Duration};

use crate::{
    api::v1::gql::{ext::RequestExt, request_context::RequestContext, schema},
    database::user,
    pb,
    tests::global::mock_global_state,
};
use async_graphql::Value;
use async_graphql::{Name, Request, Variables};
use common::prelude::FutureTimeout;
use fred::prelude::PubsubInterface;
use futures_util::StreamExt;
use prost::Message;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_serial_user_display_name_subscription() {
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

    let subscription_client =
        crate::global::setup_redis_subscription(&global.config, Default::default()).await;

    let g = global.clone();
    let handle = tokio::spawn(async move {
        g.subscription_manager
            .run(g.ctx.clone(), subscription_client)
            .await
            .unwrap();
    });

    let schema = schema();

    {
        let query = r#"
            subscription userByDisplayNameSub($userId: UUID!) {
                userDisplayName(userId: $userId) {
                    displayName
                    username
                }
            }
        "#;

        let ctx = Arc::new(RequestContext::new(false));

        let mut variables = Variables::default();
        variables.insert(Name::new("userId"), Value::from(user.id.to_string()));

        let mut stream = schema.execute_stream(
            Request::from(query)
                .variables(variables)
                .provide_global(global.clone())
                .provide_context(ctx),
        );

        let res = tokio::time::timeout(Duration::from_secs(1), stream.next())
            .await
            .expect("failed to execute stream");
        assert!(res.is_some());

        let res = res.unwrap();

        println!("{:?}", res);
        assert!(res.is_ok());
        assert_eq!(res.errors.len(), 0);
        let json = res.data.into_json();
        assert!(json.is_ok());

        assert_eq!(
            json.unwrap(),
            serde_json::json!({ "userDisplayName": { "displayName": "admin", "username": "admin" } })
        );

        // The above is the initial event send.
        // We now need to publish an event to the redis channel to trigger the subscription.
        let count: i32 = global
            .redis
            .publish(
                format!("user.{}.display_name", user.id),
                pb::scuffle::types::api::UserDisplayName {
                    display_name: Some("Admin".to_string()),
                    username: None,
                }
                .encode_to_vec()
                .as_slice(),
            )
            .await
            .expect("failed to publish to redis");

        assert_eq!(count, 1);

        let res = tokio::time::timeout(Duration::from_secs(1), stream.next())
            .await
            .expect("failed to execute stream");

        assert!(res.is_some());

        let res = res.unwrap();

        println!("{:?}", res);
        assert!(res.is_ok());
        assert_eq!(res.errors.len(), 0);
        let json = res.data.into_json();
        assert!(json.is_ok());

        assert_eq!(
            json.unwrap(),
            serde_json::json!({ "userDisplayName": { "displayName": "Admin", "username": "admin" } })
        );
    }

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Try publish to redis to see if we are still subscribed.
    let count: i32 = global
        .redis
        .publish(
            format!("user.{}.display_name", user.id),
            pb::scuffle::types::api::UserDisplayName {
                display_name: Some("Admin".to_string()),
                username: None,
            }
            .encode_to_vec()
            .as_slice(),
        )
        .await
        .expect("failed to publish to redis");
    assert_eq!(count, 0);

    drop(global);

    handler
        .cancel()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel context");

    handle.await.expect("failed to join subscription manager")
}
