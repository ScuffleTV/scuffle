use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Model {
    pub id: i64,                     // bigint, primary key
    pub user_id: i64,                // bigint, foreign key -> users.id
    pub token: String,               // char(64)
    pub created_at: DateTime<Utc>,   // timestamptz
    pub expires_at: DateTime<Utc>,   // timestamptz
    pub last_used_at: DateTime<Utc>, // timestamptz
}
