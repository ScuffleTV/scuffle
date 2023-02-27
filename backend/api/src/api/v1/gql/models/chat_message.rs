use async_graphql::{ComplexObject, Context, Enum, SimpleObject};
use uuid::Uuid;

use super::{date, user::User};
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
    pub id: Uuid,
    pub channel_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
    pub created_at: date::DateRFC3339,
    pub r#type: MessageType,
}

#[ComplexObject]
impl ChatMessage {
    pub async fn author(&self, ctx: &Context<'_>) -> Result<Option<User>> {
        let global = ctx.get_global();

        if self.author_id.is_nil() {
            return Ok(None);
        }

        let user = global
            .user_by_id_loader
            .load_one(self.author_id)
            .await
            .map_err_gql("failed to fetch user")?
            .ok_or(GqlError::NotFound.with_message("user not found"))?;

        Ok(Some(User::from(user)))
    }

    pub async fn channel(&self, ctx: &Context<'_>) -> Result<User> {
        let global = ctx.get_global();

        let user = global
            .user_by_id_loader
            .load_one(self.channel_id)
            .await
            .map_err_gql("failed to fetch user")?
            .ok_or(GqlError::NotFound.with_message("user not found"))?;

        Ok(User::from(user))
    }
}

impl From<chat_message::Model> for ChatMessage {
    fn from(model: chat_message::Model) -> Self {
        Self {
            id: model.id,
            channel_id: model.channel_id,
            author_id: model.author_id,
            content: model.content,
            created_at: model.created_at.into(),
            r#type: MessageType::User,
        }
    }
}
