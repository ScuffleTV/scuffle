use async_graphql::SimpleObject;

use super::{date, ulid::GqlUlid};

#[derive(SimpleObject)]
pub struct Session {
    /// The session's id
    pub id: GqlUlid,
    /// The session's token
    pub token: String,
    /// The user who owns this session
    pub user_id: GqlUlid,
    /// Expires at
    pub expires_at: date::DateRFC3339,
    /// Last used at
    pub last_used_at: date::DateRFC3339,
}
