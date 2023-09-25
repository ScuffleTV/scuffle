use crate::{
    api::v1::gql::ext::RequestExt,
    database::{chat_message, session, user},
};
use async_graphql::{Name, Request, Variables};
use chrono::Utc;
use common::prelude::FutureTimeout;
use prost::Message;
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    api::v1::gql::{request_context::RequestContext, schema},
    tests::global::mock_global_state,
};

#[tokio::test]
#[serial]
async fn test_serial_send_message_not_logged_in() {
    let (global, _handler) = mock_global_state(Default::default()).await;
    let schema = schema();

    let query = r#"
        mutation SendChatMessage($channelId: UUID!, $content: String!) {
            chat {
                sendMessage(channelId: $channelId, content: $content) {
                    id
                }
            }
        }
    "#;

    let ctx = Arc::new(RequestContext::default());

    let mut variables = Variables::default();
    variables.insert(
        Name::new("channelId"),
        async_graphql::Value::String(Uuid::new_v4().to_string()),
    );

    variables.insert(
        Name::new("content"),
        async_graphql::Value::String("message".to_owned()),
    );

    let res = schema
        .execute(
            Request::from(query)
                .variables(variables)
                .provide_global(global.clone())
                .provide_context(ctx),
        )
        .await;

    println!("{:?}", res);

    assert_eq!(res.errors.len(), 1);
    let json = res.data.into_json();
    assert!(json.is_ok());
    assert_eq!(json.unwrap(), serde_json::json!(null));
    assert_eq!(
        res.errors[0].message,
        "Unauthorized: You need to be logged in"
    );
}

#[tokio::test]
#[serial]
async fn test_serial_send_message_chat_not_found() {
    let (global, _handler) = mock_global_state(Default::default()).await;
    let schema = schema();
    let query = r#"
        mutation SendChatMessage($channelId: UUID!, $content: String!) {
            chat {
                sendMessage(channelId: $channelId, content: $content) {
                    id
                }
            }
        }
    "#;

    sqlx::query!("DELETE FROM sessions")
        .execute(global.db.as_ref())
        .await
        .unwrap();

    sqlx::query!("DELETE FROM users")
        .execute(global.db.as_ref())
        .await
        .unwrap();

    let user = sqlx::query_as!(user::Model,
        "INSERT INTO users(username, display_name, email, password_hash, stream_key) VALUES ($1, $1, $2, $3, $4) RETURNING *",
        "test",
        "test@test.com",
        user::hash_password("test"),
        user::generate_stream_key(),
    )
    .fetch_one(global.db.as_ref())
    .await
    .unwrap();

    let session = sqlx::query_as!(
        session::Model,
        "INSERT INTO sessions(user_id, expires_at) VALUES ($1, $2) RETURNING *",
        user.id,
        Utc::now() + chrono::Duration::seconds(120)
    )
    .fetch_one(global.db.as_ref())
    .await
    .unwrap();

    let ctx = Arc::new(RequestContext::new(false));
    ctx.set_session(Some((session, Default::default())));

    let mut variables = Variables::default();
    variables.insert(
        Name::new("channelId"),
        async_graphql::Value::String(Uuid::new_v4().to_string()),
    );

    variables.insert(
        Name::new("content"),
        async_graphql::Value::String("message".to_owned()),
    );

    let res = schema
        .execute(
            Request::from(query)
                .variables(variables)
                .provide_global(global.clone())
                .provide_context(ctx),
        )
        .await;
    assert_eq!(res.errors.len(), 1);
    let json = res.data.into_json();
    assert!(json.is_ok());
    assert_eq!(res.errors[0].message, "InvalidInput: Channel not found");
}

#[tokio::test]
#[serial]
async fn test_serial_send_message_success() {
    let (global, _handler) = mock_global_state(Default::default()).await;
    let schema = schema();
    let query = r#"
        mutation SendChatMessage($channelId: UUID!, $content: String!) {
            chat {
                sendMessage(channelId: $channelId, content: $content) {
                    id
                    content
                    author {
                        username
                    }
                    channel {
                        username
                    }
                }
            }
        }
    "#;

    sqlx::query!("DELETE FROM chat_messages")
        .execute(global.db.as_ref())
        .await
        .unwrap();

    sqlx::query!("DELETE FROM sessions")
        .execute(global.db.as_ref())
        .await
        .unwrap();

    sqlx::query!("DELETE FROM users")
        .execute(global.db.as_ref())
        .await
        .unwrap();

    let user = sqlx::query_as!(user::Model,
        "INSERT INTO users(username, display_name, email, password_hash, stream_key) VALUES ($1, $1, $2, $3, $4) RETURNING *",
        "test",
        "test@test.com",
        user::hash_password("test"),
        user::generate_stream_key(),
    )
    .fetch_one(global.db.as_ref())
    .await
    .unwrap();

    let channel = sqlx::query_as!(user::Model,
        "INSERT INTO users(username, display_name, email, password_hash, stream_key) VALUES ($1, $1, $2, $3, $4) RETURNING *",
        "based",
        "based@based.com",
        user::hash_password("based"),
        user::generate_stream_key(),
    )
    .fetch_one(global.db.as_ref())
    .await
    .unwrap();

    let session = sqlx::query_as!(
        session::Model,
        "INSERT INTO sessions(user_id, expires_at) VALUES ($1, $2) RETURNING *",
        user.id,
        Utc::now() + chrono::Duration::seconds(120)
    )
    .fetch_one(global.db.as_ref())
    .await
    .unwrap();

    let ctx = Arc::new(RequestContext::new(true));
    ctx.set_session(Some((session, Default::default())));

    let mut variables = Variables::default();
    variables.insert(
        Name::new("channelId"),
        async_graphql::Value::String(channel.id.to_string()),
    );

    variables.insert(
        Name::new("content"),
        async_graphql::Value::String("message".to_string()),
    );

    let mut subs = global
        .subscription_manager
        .subscribe(format!("user:{}:chat:messages", channel.id))
        .timeout(std::time::Duration::from_secs(1))
        .await
        .unwrap()
        .unwrap();

    let res = schema
        .execute(
            Request::from(query)
                .variables(variables)
                .provide_global(global.clone())
                .provide_context(ctx),
        )
        .await;
    assert_eq!(res.errors.len(), 0);
    let json = res.data.to_owned().into_json();
    assert!(json.is_ok());

    let json = json.unwrap();

    assert_eq!(json["chat"]["sendMessage"]["author"]["username"], "test");

    assert_eq!(json["chat"]["sendMessage"]["channel"]["username"], "based");

    assert_eq!(json["chat"]["sendMessage"]["content"], "message");

    let message = subs
        .recv()
        .timeout(std::time::Duration::from_secs(1))
        .await
        .unwrap()
        .unwrap();
    let message =
        pb::scuffle::internal::platform::events::ChatMessage::decode(message.as_bytes().unwrap())
            .unwrap();

    assert_eq!(message.author_id, user.id.to_string());
    assert_eq!(message.channel_id, channel.id.to_string());
    assert_eq!(message.content, "message");
    assert_eq!(
        message.id,
        json["chat"]["sendMessage"]["id"].as_str().unwrap()
    );

    // Check if added to db
    let db_message = sqlx::query_as!(
        chat_message::Model,
        "SELECT * FROM chat_messages WHERE id = $1",
        Uuid::parse_str(json["chat"]["sendMessage"]["id"].as_str().unwrap()).unwrap()
    )
    .fetch_one(global.db.as_ref())
    .await;

    assert!(db_message.is_ok());
    let db_message = db_message.unwrap();
    assert_eq!(db_message.author_id, user.id);
    assert_eq!(db_message.channel_id, channel.id);
    assert_eq!(db_message.content, "message");
}

#[tokio::test]
#[serial]
async fn test_serial_send_message_too_long() {
    let (global, _handler) = mock_global_state(Default::default()).await;
    let schema = schema();
    let query = r#"
        mutation SendMessage($channelId: UUID!, $content: String!) {
            chat {
                sendMessage(channelId: $channelId, content: $content) {
                    id
                }
            }
        }
    "#;

    sqlx::query!("DELETE FROM sessions")
        .execute(global.db.as_ref())
        .await
        .unwrap();

    sqlx::query!("DELETE FROM users")
        .execute(global.db.as_ref())
        .await
        .unwrap();

    let user = sqlx::query_as!(user::Model,
        "INSERT INTO users(username, display_name, email, password_hash, stream_key) VALUES ($1, $1, $2, $3, $4) RETURNING *",
        "test",
        "test@test.com",
        user::hash_password("test"),
        user::generate_stream_key(),
    )
    .fetch_one(global.db.as_ref())
    .await
    .unwrap();

    let session = sqlx::query_as!(
        session::Model,
        "INSERT INTO sessions(user_id, expires_at) VALUES ($1, $2) RETURNING *",
        user.id,
        Utc::now() + chrono::Duration::seconds(120)
    )
    .fetch_one(global.db.as_ref())
    .await
    .unwrap();

    let ctx = Arc::new(RequestContext::new(true));
    ctx.set_session(Some((session, Default::default())));

    let mut variables = Variables::default();
    variables.insert(
        Name::new("channelId"),
        async_graphql::Value::String(Uuid::new_v4().to_string()),
    );
    variables.insert(
        Name::new("content"),
        async_graphql::Value::String("a".repeat(501)),
    );

    let res = schema
        .execute(
            Request::from(query)
                .variables(variables)
                .provide_global(global.clone())
                .provide_context(ctx),
        )
        .await;

    assert_eq!(res.errors.len(), 1);
    let json = res.data.into_json();
    assert!(json.is_ok());
    assert_eq!(res.errors[0].message, "InvalidInput: Message too long");
}
