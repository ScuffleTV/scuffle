use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Default, Copy, PartialEq, Eq)]
#[repr(i64)]
pub enum Level {
    #[default]
    Info = 0,
    Warning = 1,
    Error = 2,
}

impl From<i64> for Level {
    fn from(value: i64) -> Self {
        match value {
            0 => Self::Info,
            1 => Self::Warning,
            2 => Self::Error,
            _ => Self::Info,
        }
    }
}

impl From<Level> for i64 {
    fn from(value: Level) -> Self {
        match value {
            Level::Info => 0,
            Level::Warning => 1,
            Level::Error => 2,
        }
    }
}

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct Model {
    /// The unique identifier for the stream variant.
    pub id: Uuid,
    /// The unique identifier for the stream.
    pub stream_id: Uuid,
    pub title: String,
    pub message: String,
    pub level: Level,
    pub created_at: DateTime<Utc>,
}
