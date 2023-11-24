use async_graphql::{Context, Subscription};
use async_stream::stream;
use futures_util::Stream;
use pb::ext::*;
use prost::Message;
use ulid::Ulid;

use crate::api::v1::gql::error::ext::*;
use crate::api::v1::gql::error::Result;
use crate::api::v1::gql::ext::ContextExt;
use crate::api::v1::gql::models::chat_message::{ChatMessage, MessageType};
use crate::api::v1::gql::models::ulid::GqlUlid;
use crate::global::ApiGlobal;

pub struct ChatSubscription<G: ApiGlobal>(std::marker::PhantomData<G>);

impl<G: ApiGlobal> Default for ChatSubscription<G> {
	fn default() -> Self {
		Self(std::marker::PhantomData)
	}
}

#[Subscription]
impl<G: ApiGlobal> ChatSubscription<G> {
	// Listen to new messages in chat.
	pub async fn chat_messages<'ctx>(
		&self,
		ctx: &'ctx Context<'_>,
		#[graphql(desc = "Chat to subscribe to.")] channel_id: GqlUlid,
	) -> Result<impl Stream<Item = Result<ChatMessage<G>>> + 'ctx> {
		let global = ctx.get_global::<G>();

		let welcome_message = ChatMessage {
			id: Ulid::nil().into(),
			user_id: Ulid::nil().into(),
			channel_id,
			content: "Welcome to the chat!".to_string(),
			r#type: MessageType::Welcome,
			_phantom: std::marker::PhantomData,
		};

		// TODO: check if user is allowed to read this chat
		// let channel = global
		//     .user_by_id_loader
		//     .load_one(channel_id.into())
		//     .await
		//     .map_err_gql("failed to fetch user")?
		//     .ok_or(GqlError::NotFound.with_message("user not found"))?;

		let mut message_stream = global
			.subscription_manager()
			.subscribe(format!("channel.{}.chat.messages", *channel_id))
			.await
			.map_err_gql("failed to subscribe to chat messages")?;

		Ok(stream!({
			yield Ok(welcome_message);
			while let Ok(message) = message_stream.recv().await {
				let event = pb::scuffle::platform::internal::events::ChatMessage::decode(message.payload)
					.map_err_ignored_gql("failed to decode chat message")?;

				yield Ok(ChatMessage {
					id: event.id.into_ulid().into(),
					user_id: event.user_id.into_ulid().into(),
					channel_id: event.channel_id.into_ulid().into(),
					content: event.content,
					r#type: MessageType::User,
					_phantom: std::marker::PhantomData,
				});
			}
		}))
	}
}
