use async_graphql::{ComplexObject, Context, Enum, SimpleObject};

use super::ulid::GqlUlid;
use super::user::User;
use crate::api::v1::gql::error::ext::*;
use crate::api::v1::gql::error::{GqlError, Result};
use crate::api::v1::gql::ext::ContextExt;
use crate::database;
use crate::global::ApiGlobal;

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum MessageType {
	User,
	Welcome,
	System,
}

#[derive(SimpleObject)]
#[graphql(complex)]
pub struct ChatMessage<G: ApiGlobal> {
	pub id: GqlUlid,
	pub channel_id: GqlUlid,
	pub user_id: GqlUlid,
	pub content: String,
	pub r#type: MessageType,

	#[graphql(skip)]
	pub _phantom: std::marker::PhantomData<G>,
}

#[ComplexObject]
impl<G: ApiGlobal> ChatMessage<G> {
	pub async fn user(&self, ctx: &Context<'_>) -> Result<Option<User<G>>> {
		let global = ctx.get_global::<G>();

		if self.user_id.is_nil() {
			return Ok(None);
		}

		let user = global
			.user_by_id_loader()
			.load(self.user_id.into())
			.await
			.map_err_ignored_gql("failed to fetch user")?
			.ok_or(GqlError::NotFound("user"))?;

		Ok(Some(user.into()))
	}

	pub async fn channel(&self, ctx: &Context<'_>) -> Result<User<G>> {
		let global = ctx.get_global::<G>();

		let user = global
			.user_by_id_loader()
			.load(self.channel_id.into())
			.await
			.map_err_ignored_gql("failed to fetch user")?
			.ok_or(GqlError::NotFound("user"))?;

		Ok(user.into())
	}
}

impl<G: ApiGlobal> From<database::ChatMessage> for ChatMessage<G> {
	fn from(model: database::ChatMessage) -> Self {
		Self {
			id: model.id.into(),
			channel_id: model.channel_id.into(),
			user_id: model.user_id.into(),
			content: model.content,
			r#type: MessageType::User,
			_phantom: std::marker::PhantomData,
		}
	}
}
