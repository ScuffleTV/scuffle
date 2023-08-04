use crate::api::v1::gql::error::ResultExt;
use crate::database::chat_message;
use prost::Message;

use super::error::{GqlError, Result};
use super::ext::ContextExt;
use super::models::chat_message::ChatMessage;
use async_graphql::{Context, Object};
use fred::prelude::PubsubInterface;
use uuid::Uuid;

const MAX_MESSAGE_LENGTH: usize = 500;

#[derive(Default)]
pub struct ChatMutation;

#[Object]
impl ChatMutation {
    // Send message in chat. You need to be logged in for that.
    async fn send_message<'ctx>(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "ID of chat room where the message will be send.")] channel_id: Uuid,
        #[graphql(desc = "Message content that will be published.")] content: String,
    ) -> Result<ChatMessage> {
        let global = ctx.get_global();
        let request_context = ctx.get_session();

        if content.len() > MAX_MESSAGE_LENGTH {
            return Err(GqlError::InvalidInput.with_message("Message too long"));
        }

        // TODO: check if user is banned from chat
        let (session, _) = request_context
            .get_session(global)
            .await?
            .ok_or_else(|| GqlError::Unauthorized.with_message("You need to be logged in"))?;

        // TODO: Check if the user is allowed to send messages in this chat
        let channel = global
            .user_by_id_loader
            .load_one(channel_id)
            .await
            .map_err_gql("Failed to fetch channel")?
            .ok_or_else(|| GqlError::InvalidInput.with_message("Channel not found"))?;

        let chat_message = sqlx::query_as!(
            chat_message::Model,
            "INSERT INTO chat_messages (channel_id, author_id, content) VALUES ($1, $2, $3) RETURNING *",
            channel.id,
            session.user_id,
            content,
        ).fetch_one(&*global.db).await.map_err_gql("Failed to insert chat message")?;

        match global
            .redis
            .publish(
                format!("user:{}:chat:messages", channel.id),
                pb::scuffle::internal::platform::events::ChatMessage {
                    id: chat_message.id.to_string(),
                    channel_id: chat_message.channel_id.to_string(),
                    author_id: chat_message.author_id.to_string(),
                    content: chat_message.content.clone(),
                    created_at: chat_message.created_at.timestamp(),
                }
                .encode_to_vec()
                .as_slice(),
            )
            .await
        {
            Ok(()) => {}
            Err(_) => {
                return Err(GqlError::InternalServerError.with_message("Failed to publish message"));
            }
        };

        Ok(chat_message.into())
    }
}
