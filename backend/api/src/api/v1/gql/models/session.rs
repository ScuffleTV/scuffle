use async_graphql::{ComplexObject, Context, SimpleObject};
use uuid::Uuid;

use super::{date, user::User};
use crate::api::v1::gql::{
    error::{GqlError, Result, ResultExt},
    ext::ContextExt,
};

#[derive(SimpleObject)]
#[graphql(complex)]
pub struct Session {
    /// The session's id
    pub id: Uuid,
    /// The session's token
    pub token: String,
    /// The user who owns this session
    pub user_id: Uuid,
    /// Expires at
    pub expires_at: date::DateRFC3339,
    /// Last used at
    pub last_used_at: date::DateRFC3339,
    /// Created at
    pub created_at: date::DateRFC3339,

    #[graphql(skip)]
    pub _user: Option<User>,
}

#[ComplexObject]
impl Session {
    pub async fn user(&self, ctx: &Context<'_>) -> Result<User> {
        if let Some(user) = &self._user {
            return Ok(user.clone());
        }

        let global = ctx.get_global();

        let user = global
            .user_by_id_loader
            .load_one(self.user_id)
            .await
            .map_err_gql("failed to fetch user")?
            .ok_or(GqlError::NotFound.with_message("user not found"))?;

        Ok(User::from(user))
    }
}
