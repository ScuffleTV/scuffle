use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Default)]
pub struct Model {
    pub id: i64,                      // bigint, primary key
    pub username: String,             // varchar(32)
    pub password_hash: String,        // varchar(255)
    pub email: String,                // varchar(255)
    pub email_verified: bool,         // bool
    pub created_at: DateTime<Utc>,    // timestamptz
    pub last_login_at: DateTime<Utc>, // timestamptz
}
