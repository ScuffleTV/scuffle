use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Model {
    pub id: i64,                           // bigint, primary key
    pub channel_id: i64,                   // bigint, foreign key -> channels.id
    pub title: String,                     // varchar(255)
    pub description: String,               // text
    pub created_at: DateTime<Utc>,         // timestamptz
    pub started_at: Option<DateTime<Utc>>, // timestamptz?
    pub ended_at: Option<DateTime<Utc>>,   // timestamptz?
}
