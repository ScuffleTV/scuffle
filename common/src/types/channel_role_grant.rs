use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Default)]
pub struct Model {
    pub id: i64,                   // bigint, primary key
    pub user_id: i64,              // bigint, foreign key -> users.id
    pub channel_role_id: i64,      // bigint, foreign key -> channel_roles.id
    pub created_at: DateTime<Utc>, // timestamptz
}
