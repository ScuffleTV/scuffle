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
use crate::database;
use crate::global::ApiGlobal;
use crate::subscription::SubscriptionTopic;

pub struct ChatSubscription<G: ApiGlobal>(std::marker::PhantomData<G>);

impl<G: ApiGlobal> Default for ChatSubscription<G> {
	fn default() -> Self {
		Self(std::marker::PhantomData)
	}
}

#[Subscription]
impl<G: ApiGlobal> ChatSubscription<G> {
	// Listen to new messages in chat.
	async fn chat_messages<'ctx>(
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

		let mut message_stream = global
			.subscription_manager()
			.subscribe(SubscriptionTopic::ChannelChatMessages(channel_id.to_ulid()))
			.await
			.map_err_gql("failed to subscribe to chat messages")?;

		// load old messages not older than 10 minutes, max 100 messages
		let not_older_than = chrono::Utc::now() - chrono::Duration::minutes(10);
		let not_older_than = ulid::Ulid::from_parts(not_older_than.timestamp() as u64, u128::MAX);
		let messages: Vec<database::ChatMessage> = sqlx::query_as(
			"SELECT * FROM chat_messages WHERE channel_id = $1 AND deleted_at IS NULL AND id >= $2 ORDER BY id LIMIT 100",
		)
		.bind(common::database::Ulid::from(channel_id.to_ulid()))
		.bind(common::database::Ulid::from(not_older_than))
		.fetch_all(global.db().as_ref())
		.await
		.map_err_gql("failed to fetch chat messages")?;

		Ok(stream!({
			for message in messages {
				yield Ok(message.into());
			}
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
