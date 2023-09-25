use async_graphql::{ComplexObject, Context, Enum, SimpleObject};

use super::{ulid::GqlUlid, user::User};
use crate::{
    api::v1::gql::{
        error::{GqlError, Result, ResultExt},
        ext::ContextExt,
    },
    database,
};

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum MessageType {
    User,
    Welcome,
    System,
}

#[derive(SimpleObject)]
#[graphql(complex)]
pub struct ChatMessage {
    pub id: GqlUlid,
    pub channel_id: GqlUlid,
    pub user_id: GqlUlid,
    pub content: String,
    pub r#type: MessageType,
}

#[ComplexObject]
impl ChatMessage {
    pub async fn user(&self, ctx: &Context<'_>) -> Result<Option<User>> {
        let global = ctx.get_global();

        if self.user_id.is_nil() {
            return Ok(None);
        }

        let user = global
            .user_by_id_loader
            .load(self.user_id.into())
            .await
            .map_err_gql("failed to fetch user")?
            .ok_or(GqlError::NotFound("user"))?;

        Ok(Some(User::from(user)))
    }

    pub async fn channel(&self, ctx: &Context<'_>) -> Result<User> {
        let global = ctx.get_global();

        let user = global
            .user_by_id_loader
            .load(self.channel_id.into())
            .await
            .map_err_gql("failed to fetch user")?
            .ok_or(GqlError::NotFound("user"))?;

        Ok(User::from(user))
    }
}

impl From<database::ChatMessage> for ChatMessage {
    fn from(model: database::ChatMessage) -> Self {
        Self {
            id: model.id.0.into(),
            channel_id: model.channel_id.0.into(),
            user_id: model.user_id.0.into(),
            content: model.content,
            r#type: MessageType::User,
        }
    }
}
