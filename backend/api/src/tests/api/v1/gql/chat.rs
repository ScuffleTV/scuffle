use arc_swap::ArcSwap;
use async_graphql::{Name, Request, Variables};
use chrono::Utc;
use common::types::{session, user};
use std::sync::Arc;

use crate::{
    api::v1::gql::{schema, GqlContext},
    tests::global::mock_global_state,
};

#[tokio::test]
async fn test_send_message() {
    // Set up global state
    let (global, _handler) = mock_global_state(Default::default()).await;

    sqlx::query!("DELETE FROM chat_messages")
        .execute(&*global.db)
        .await
        .unwrap();

    sqlx::query!("DELETE FROM chat_rooms")
        .execute(&*global.db)
        .await
        .unwrap();

    sqlx::query!("DELETE FROM sessions")
        .execute(&*global.db)
        .await
        .unwrap();

    // not logged in
    let schema = schema();

    let query = r#"
        mutation {
            chat {
                sendMessage(chatId: 1, content: "message")
            }
        }
    "#;

    let ctx = Arc::new(GqlContext::default());

    let res = schema
        .execute(Request::from(query).data(global.clone()).data(ctx))
        .await;

    assert_eq!(res.errors.len(), 1);
    let json = res.data.into_json();
    assert!(json.is_ok());

    // Chat not found
    sqlx::query!(
        "INSERT INTO users(id, username, email, password_hash) VALUES ($1, $2, $3, $4)",
        1,
        "test",
        "test@test.com",
        user::hash_password("test")
    )
    .execute(&*global.db)
    .await
    .unwrap();

    let session = sqlx::query_as!(
        session::Model,
        "INSERT INTO sessions(id, user_id, expires_at) VALUES ($1, $2, $3) RETURNING *",
        1,
        1,
        Utc::now() + chrono::Duration::seconds(120)
    )
    .fetch_one(&*global.db)
    .await
    .unwrap();

    let ctx = Arc::new(GqlContext {
        is_websocket: false,
        session: ArcSwap::from_pointee(Some(session.to_owned())),
    });

    let res = schema
        .execute(Request::from(query).data(global.clone()).data(ctx))
        .await;
    assert_eq!(res.errors.len(), 1);
    let json = res.data.into_json();
    assert!(json.is_ok());

    // Add message
    sqlx::query!(
        "INSERT INTO chat_rooms(id, owner_id, name, description) VALUES ($1, $2, $3, $4)",
        1,
        1,
        "name",
        "description"
    )
    .execute(&*global.db)
    .await
    .unwrap();

    let ctx = Arc::new(GqlContext {
        is_websocket: true,
        session: ArcSwap::from_pointee(Some(session.to_owned())),
    });

    let res = schema
        .execute(Request::from(query).data(global.clone()).data(ctx))
        .await;
    assert_eq!(res.errors.len(), 0);
    let json = res.data.into_json();
    assert!(json.is_ok());

    // Check if added in db
    let db_result = sqlx::query!("SELECT * FROM chat_messages WHERE chat_room_id = $1", 1)
        .fetch_one(&*global.db)
        .await;

    assert!(db_result.is_ok());
    let db_result = db_result.unwrap();
    assert_eq!(db_result.chat_room_id, 1);
    assert_eq!(db_result.author_id, 1);
    assert_eq!(db_result.message, "message");

    let ctx = Arc::new(GqlContext {
        is_websocket: true,
        session: ArcSwap::from_pointee(Some(session.to_owned())),
    });

    let query = r#"
        mutation SendMessage($content: String!) {
            chat {
                sendMessage(chatId: 1, content: $content)
            }
        }
    "#;

    // Message too long
    let mut variables = Variables::default();
    variables.insert(
        Name::new("content"),
        async_graphql::Value::String("a".repeat(501)),
    );

    let res = schema
        .execute(
            Request::from(query)
                .variables(variables)
                .data(global.clone())
                .data(ctx),
        )
        .await;

    assert_eq!(res.errors.len(), 1);
    let json = res.data.into_json();
    assert!(json.is_ok());
}
