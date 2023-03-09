use super::error::{GqlError, Result, ResultExt};
use super::models::message::Message;
use crate::api::v1::gql::GqlContext;
use crate::global::GlobalState;
use async_graphql::{futures_util::Stream, Context, Object, Subscription};
use async_stream::stream;
use chrono::Utc;
use fred::prelude::PubsubInterface;
use std::{collections::HashMap, sync::Arc};

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

        let user = global
            .user_by_id_loader
            .load_one(user_id)
            .await
            .map_err_gql("Failed to fetch user")?
            .ok_or(GqlError::InvalidInput.with_message("User not found"))?;

        let _chat = global
            .chat_room_by_id_loader
            .load_one(chat_id)
            .await
            .map_err_gql("Failed to fetch chat")?
            .ok_or(GqlError::InvalidInput.with_message("Chat not found"))?;

        let message = Message {
            chat_id,
            username: user.username,
            content: content.clone(),
            message_type: "message".to_string(),
            metadata: HashMap::new(),
        };

        let message_to_send = serde_json::json!(message).to_string();
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
    ) -> Result<impl Stream<Item = Message>> {
        let global = ctx
            .data::<Arc<GlobalState>>()
            .expect("failed to get global state")
            .clone();

        let mut message_stream = global.redis_sub_client.on_message();
        let _ = global
            .redis_sub_client
            .subscribe::<String, String>("chat:".to_string() + &chat_id.to_string())
            .await;

        Ok(stream! {
            while let Ok(message) = message_stream.recv().await {
                if message.channel.split(':').collect::<Vec<&str>>()[1] == chat_id.to_string() {
                    let data = serde_json::from_str::<Message>(&message.value.as_str().unwrap());
                    if data.is_ok() {
                        yield data.unwrap();
                    }
                }
            }
        })
    }
}
