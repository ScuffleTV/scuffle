use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Model {
    pub id: i64,                   // bigint, primary key
    pub follower_id: i64,          // bigint, foreign key -> users.id
    pub followed_id: i64,          // bigint, foreign key -> users.id
    pub channel_id: Option<i64>,   // bigint, foreign key -> channels.id
    pub created_at: DateTime<Utc>, // timestamptz
}
