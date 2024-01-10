use common::database::Ulid;

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct ChatMessage {
	/// The unique identifier for the chat message.
	pub id: Ulid,
	/// The unique identifier for the user who sent the message.
	pub user_id: Ulid,
	/// The unique identifier for the chat room which owns the message.
	pub channel_id: Ulid,
	/// The content of the message.
	pub content: String,
	/// The time the message was deleted.
	pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl ChatMessage {
	pub fn to_protobuf(&self) -> pb::scuffle::platform::internal::events::ChatMessage {
		pb::scuffle::platform::internal::events::ChatMessage {
			id: Some(self.id.0.into()),
			channel_id: Some(self.channel_id.0.into()),
			user_id: Some(self.user_id.0.into()),
			content: self.content.clone(),
		}
	}
}
