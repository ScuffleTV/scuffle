use ulid::Ulid;
use uuid::Uuid;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct Model {
    /// The unique identifier for the chat message.
    pub id: Uuid,
    /// The unique identifier for the chat room which owns the message.
    pub channel_id: Uuid,
    /// The unique identifier for the user who sent the message.
    pub user_id: Uuid,
    /// The content of the message.
    pub content: String,
}

impl Model {
    pub fn to_protobuf(&self) -> pb::scuffle::platform::internal::events::ChatMessage {
        pb::scuffle::platform::internal::events::ChatMessage {
            id: Ulid::from(self.id).to_string(),
            channel_id: Ulid::from(self.channel_id).to_string(),
            user_id: Ulid::from(self.user_id).to_string(),
            content: self.content.clone(),
        }
    }
}
