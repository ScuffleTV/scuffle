use async_graphql::SimpleObject;

use super::date;

#[derive(SimpleObject)]
pub struct Session {
    /// The session's id
    pub id: i64,
    /// The session's token
    pub token: String,
    /// The user who owns this session
    pub user_id: i64,
    /// Expires at
    pub expires_at: date::DateRFC3339,
    /// Last used at
    pub last_used_at: date::DateRFC3339,
    /// Created at
    pub created_at: date::DateRFC3339,
}
