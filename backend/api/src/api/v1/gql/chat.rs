use super::error::{GqlError, Result};
use super::models::message::Message;
use crate::api::v1::gql::GqlContext;
use crate::global::GlobalState;
use async_graphql::{
    futures_util::{Stream, StreamExt},
    Context, Object, Subscription,
};
use async_stream::stream;
use chrono::Utc;
use common::types::{chat_room, user};
use fred::prelude::PubsubInterface;
use std::sync::Arc;

#[derive(Default)]
pub struct ChatMutation;

#[derive(Default)]
pub struct ChatSubscription;

#[Object]
impl ChatMutation {
    // Send message in chat. You need to be logged in for that.
    async fn send_message<'ctx>(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "ID of chat room where the message will be send.")] chat_id: i64,
        #[graphql(desc = "Message content that will be published.")] content: String,
    ) -> Result<String> {
        let global = ctx
            .data::<Arc<GlobalState>>()
            .expect("failed to get global state")
            .clone();

        let request_context = ctx
            .data::<Arc<GqlContext>>()
            .expect("failed to get request context");

        if content.len() > 500 {
            return Err(GqlError::InvalidInput.with_message("Message too long"));
        }

        let session = request_context.get_session(&global).await?;

        let user_id = session
            .as_ref()
            .ok_or_else(|| GqlError::InvalidInput.with_message("You need to be logged in"))?
            .user_id;

        let user = sqlx::query!("SELECT username FROM users WHERE id = $1", user_id,)
            .map(|row| user::Model {
                username: row.username,
                ..Default::default()
            })
            .fetch_optional(&*global.db)
            .await
            .map_err(|_| GqlError::InvalidInput.with_message("Failed to fetch user"))?
            .ok_or_else(|| GqlError::InvalidInput.with_message("User not found"))?;

        let _chat = sqlx::query!("SELECT id FROM chat_rooms WHERE id = $1", chat_id,)
            .map(|row| chat_room::Model {
                id: row.id,
                ..Default::default()
            })
            .fetch_optional(&*global.db)
            .await
            .map_err(|_| GqlError::InvalidInput.with_message("Failed to fetch chat room"))?
            .ok_or_else(|| GqlError::InvalidInput.with_message("Chat not found"))?;

        let message_to_send = serde_json::json!({ "chat_id": chat_id, "username": user.username, "content": content, "message_type": "message" }).to_string();
        match global
            .redis_pool
            .publish::<String, String, String>(
                "chat:".to_string() + &chat_id.to_string(),
                message_to_send.to_string(),
            )
            .await
        {
            Ok(_) => {}
            Err(_) => {
                return Err(GqlError::InternalServerError.with_message("Failed to publish message"));
            }
        };

        let _ = sqlx::query!(
            "INSERT INTO chat_messages (chat_room_id, author_id, message, created_at) VALUES ($1, $2, $3, $4)",
            chat_id,
            user_id,
            content,
            Utc::now()
        )
        .execute(&*global.db)
        .await;

        Ok(message_to_send)
    }
}

#[Subscription]
impl ChatSubscription {
    // Listen to new messages in chat.
    pub async fn new_message<'ctx>(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Chat to subscribe to.")] chat_id: i64,
    ) -> impl Stream<Item = Message> {
        let global = ctx
            .data::<Arc<GlobalState>>()
            .expect("failed to get global state")
            .clone();

        let mut message_stream = global.redis_sub_client.on_message();
        let _ = global
            .redis_sub_client
            .subscribe("chat:".to_string() + &chat_id.to_string())
            .await;

        stream! {
            loop {
                while let Some((channel, message)) = message_stream.next().await {
                    if channel.split(':').collect::<Vec<&str>>()[1] == chat_id.to_string() {
                        let data = serde_json::from_str::<Message>(&message.as_str().unwrap());
                        if data.is_ok() {
                            yield data.unwrap();
                        }
                    }
                }
            }
        }
    }
}
