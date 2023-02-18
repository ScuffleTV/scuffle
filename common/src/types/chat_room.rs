use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Model {
    pub id: i64,                           // bigint, primary key
    pub owner_id: i64,                     // bigint, foreign key -> users.id
    pub name: String,                      // varchar(32)
    pub description: String,               // text
    pub deleted_at: Option<DateTime<Utc>>, // timestamptz?
    pub created_at: DateTime<Utc>,         // timestamptz
}
