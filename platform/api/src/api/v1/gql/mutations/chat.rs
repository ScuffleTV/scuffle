use async_graphql::{Context, Object};
use prost::Message;
use tracing::error;
use ulid::Ulid;
use uuid::Uuid;

use crate::{
    api::v1::gql::{
        error::{GqlError, Result, ResultExt},
        ext::ContextExt,
        models::chat_message::ChatMessage,
        models::ulid::GqlUlid,
    },
    database,
};

const MAX_MESSAGE_LENGTH: usize = 500;

#[derive(Default)]
pub struct ChatMutation;

#[Object]
impl ChatMutation {
    // Send message in chat. You need to be logged in for that.
    async fn send_message<'ctx>(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "ID of chat room where the message will be send.")] channel_id: GqlUlid,
        #[graphql(desc = "Message content that will be published.")] content: String,
    ) -> Result<ChatMessage> {
        let global = ctx.get_global();
        let request_context = ctx.get_req_context();

        if content.len() > MAX_MESSAGE_LENGTH {
            return Err(GqlError::InvalidInput {
                fields: vec!["content"],
                message: "Message is too long",
            }
            .into());
        }

        // TODO: check if user is banned from chat
        let auth = request_context
            .auth()
            .await
            .map_err_gql(GqlError::NotLoggedIn)?;

        // TODO: Check if the user is allowed to send messages in this chat
        let message_id = Ulid::new();
        let chat_message: database::ChatMessage = sqlx::query_as(
            "INSERT INTO chat_messages (id, user_id, channel_id, content) VALUES ($1, $2, $3, $4) RETURNING *"
        )
        .bind(Uuid::from(message_id))
        .bind(auth.session.user_id)
        .bind(channel_id.to_uuid())
        .bind(content.clone())
        .fetch_one(global.db.as_ref())
        .await
        .map_err_gql("failed to insert chat message")?;

        match global
            .nats
            .publish(
                format!("channel.{}.chat.messages", channel_id.to_ulid()),
                chat_message.to_protobuf().encode_to_vec().into(),
            )
            .await
        {
            Ok(_) => {}
            Err(e) => {
                error!("failed to publish nats message: {}", e);
                return Err(GqlError::InternalServerError("Failed to publish message").into());
            }
        };

        Ok(chat_message.into())
    }
}
