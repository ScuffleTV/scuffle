use bitmask_enum::bitmask;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Model {
    pub id: i64,                           // bigint, primary key
    pub owner_id: i64,                     // bigint, foreign key -> users.id
    pub target_id: i64,                    // bigint, foreign key -> users.id
    pub channel_id: Option<i64>,           // bigint?, foreign key -> channels.id
    pub mode: i64,                         // bigint, bitfield -> Mode
    pub reason: String,                    // varchar(255)
    pub expires_at: Option<DateTime<Utc>>, // timestamptz?
    pub created_at: DateTime<Utc>,         // timestamptz
}

#[bitmask(i64)]
pub enum Mode {
    ChatBan,  // User is unable to type in chat
    ReadBan,  // User is unable to read chat
    WatchBan, // User is unable to watch the channel
}
