use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct Model {
    /// The unique identifier for the chat message.
    pub id: Uuid,
    /// The unique identifier for the chat room which owns the message.
    pub channel_id: Uuid,
    /// The unique identifier for the user who sent the message.
    pub author_id: Uuid,
    /// The content of the message.
    pub content: String,
    /// The time the message was created.
    pub created_at: DateTime<Utc>,
}
