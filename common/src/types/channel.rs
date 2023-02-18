use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Model {
    pub id: i64,                           // bigint, primary key
    pub owner_id: i64,                     // bigint, foreign key -> users.id
    pub name: String,                      // varchar(32)
    pub description: String,               // text
    pub stream_key: String,                // char(25)
    pub chat_room_id: Option<i64>,         // bigint?, foreign key -> chat_rooms.id
    pub last_live: Option<DateTime<Utc>>,  // timestamptz?
    pub created_at: DateTime<Utc>,         // timestamptz
    pub deleted_at: Option<DateTime<Utc>>, // timestamptz?
}
