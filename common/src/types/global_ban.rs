use bitmask_enum::bitmask;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Default)]
pub struct Model {
    pub id: i64,                           // bigint, primary key
    pub user_id: i64,                      // bigint, foreign key -> users.id
    pub mode: i64,                         // bigint, bitfield -> Mode
    pub reason: String,                    // text
    pub expires_at: Option<DateTime<Utc>>, // timestamptz?
    pub created_at: DateTime<Utc>,         // timestamptz
}

#[bitmask(i64)]
pub enum Mode {
    SiteBan,    // User is unable to login (implies all other bans)
    ChatBan,    // User is unable to type in chat
    LiveBan,    // User is banned from going live
    ReportBan,  // User is banned from using the report system
    SupportBan, // User is banned from using the support system
}
