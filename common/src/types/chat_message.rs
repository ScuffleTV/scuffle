use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Default)]
pub struct Model {
    pub id: i64,                   // bigint, primary key
    pub chat_room_id: i64,         // bigint, foreign key -> chat_rooms.id
    pub author_id: i64,            // bigint, foreign key -> users.id
    pub message: String,           // text
    pub created_at: DateTime<Utc>, // timestamptz
}
