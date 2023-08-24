use async_graphql::{Context, Subscription};
use async_stream::stream;
use chrono::{TimeZone, Utc};
use futures_util::Stream;
use prost::Message;
use uuid::Uuid;

use crate::api::v1::gql::{
    error::{GqlError, Result, ResultExt},
    ext::ContextExt,
    models::chat_message::{ChatMessage, MessageType},
};

#[derive(Default)]
pub struct ChatSubscription;

#[Subscription]
impl ChatSubscription {
    // Listen to new messages in chat.
    pub async fn chat_messages<'ctx>(
        &self,
        ctx: &'ctx Context<'_>,
        #[graphql(desc = "Chat to subscribe to.")] channel_id: Uuid,
    ) -> Result<impl Stream<Item = Result<ChatMessage>> + 'ctx> {
        let global = ctx.get_global();

        let welcome_message = ChatMessage {
            id: Uuid::nil(),
            author_id: Uuid::nil(),
            channel_id,
            content: "Welcome to the chat!".to_string(),
            created_at: chrono::Utc::now().into(),
            r#type: MessageType::Welcome,
        };

        // TODO: check if user is allowed to read this chat
        let channel = global
            .user_by_id_loader
            .load_one(channel_id)
            .await
            .map_err_gql("failed to fetch user")?
            .ok_or(GqlError::NotFound.with_message("user not found"))?;
        let mut message_stream = global
            .subscription_manager
            .subscribe(format!("user:{}:chat:messages", channel.id))
            .await
            .map_err_gql("failed to subscribe to chat messages")?;

        Ok(stream!({
            yield Ok(welcome_message);
            while let Ok(message) = message_stream.recv().await {
                todo!()
                // let event = pb::scuffle::internal::platform::events::ChatMessage::decode(
                //     message.as_bytes().map_err_gql("invalid redis value type")?,
                // )
                // .map_err_gql("failed to decode chat message")?;

                // yield Ok(ChatMessage {
                //     id: Uuid::parse_str(&event.id)
                //         .map_err_gql("failed to parse chat message id")?,
                //     author_id: Uuid::parse_str(&event.author_id)
                //         .map_err_gql("failed to parse chat message author id")?,
                //     channel_id: Uuid::parse_str(&event.channel_id)
                //         .map_err_gql("failed to parse chat message channel id")?,
                //     content: event.content,
                //     created_at: Utc
                //         .timestamp_opt(event.created_at, 0)
                //         .single()
                //         .map_err_gql("failed to parse chat message created at")?
                //         .into(),
                //     r#type: MessageType::User,
                // });
            }
        }))
    }
}
