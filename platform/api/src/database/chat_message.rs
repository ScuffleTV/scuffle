use super::Ulid;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct ChatMessage {
    /// The unique identifier for the chat message.
    pub id: Ulid,
    /// The unique identifier for the chat room which owns the message.
    pub channel_id: Ulid,
    /// The unique identifier for the user who sent the message.
    pub user_id: Ulid,
    /// The content of the message.
    pub content: String,
}

impl ChatMessage {
    pub fn to_protobuf(&self) -> pb::scuffle::platform::internal::events::ChatMessage {
        pb::scuffle::platform::internal::events::ChatMessage {
            id: self.id.0.to_string(),
            channel_id: self.channel_id.0.to_string(),
            user_id: self.user_id.0.to_string(),
            content: self.content.clone(),
        }
    }
}
