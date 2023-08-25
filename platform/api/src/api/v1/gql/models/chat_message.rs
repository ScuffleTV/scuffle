use async_graphql::{ComplexObject, Context, Enum, SimpleObject};

use super::{ulid::GqlUlid, user::User};
use crate::{
    api::v1::gql::{
        error::{GqlError, Result, ResultExt},
        ext::ContextExt,
    },
    database::chat_message,
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
            .load_one(self.user_id.into())
            .await
            .map_err_gql("failed to fetch user")?
            .ok_or(GqlError::NotFound.with_message("user not found"))?;

        Ok(Some(User::from(user)))
    }

    pub async fn channel(&self, ctx: &Context<'_>) -> Result<User> {
        let global = ctx.get_global();

        let user = global
            .user_by_id_loader
            .load_one(self.channel_id.into())
            .await
            .map_err_gql("failed to fetch user")?
            .ok_or(GqlError::NotFound.with_message("user not found"))?;

        Ok(User::from(user))
    }
}

impl From<chat_message::Model> for ChatMessage {
    fn from(model: chat_message::Model) -> Self {
        Self {
            id: model.id.into(),
            channel_id: model.channel_id.into(),
            user_id: model.user_id.into(),
            content: model.content,
            r#type: MessageType::User,
        }
    }
}
