use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Model {
    pub id: i64,                   // bigint, primary key
    pub user_id: i64,              // bigint, foreign key -> users.id
    pub global_role_id: i64,       // bigint, foreign key -> global_roles.id
    pub created_at: DateTime<Utc>, // timestamptz
}
