use async_graphql::{Context, Object};
use prost::Message;
use tracing::error;
use ulid::Ulid;

use crate::api::auth::AuthError;
use crate::api::v1::gql::error::ext::*;
use crate::api::v1::gql::error::{GqlError, Result};
use crate::api::v1::gql::ext::ContextExt;
use crate::api::v1::gql::models::chat_message::ChatMessage;
use crate::api::v1::gql::models::ulid::GqlUlid;
use crate::database;
use crate::global::ApiGlobal;
use crate::subscription::SubscriptionTopic;

pub struct ChatMutation<G>(std::marker::PhantomData<G>);

impl<G: ApiGlobal> Default for ChatMutation<G> {
	fn default() -> Self {
		Self(std::marker::PhantomData)
	}
}

#[Object]
impl<G: ApiGlobal> ChatMutation<G> {
	// Send message in chat. You need to be logged in for that.
	async fn send_message<'ctx>(
		&self,
		ctx: &Context<'_>,
		#[graphql(desc = "ID of chat room where the message will be send.")] channel_id: GqlUlid,
		#[graphql(desc = "Message content that will be published.", validator(max_length = 500))] content: String,
	) -> Result<ChatMessage<G>> {
		let global = ctx.get_global::<G>();
		let request_context = ctx.get_req_context();

		// TODO: check if user is banned from chat
		let auth = request_context
			.auth(global)
			.await?
			.map_err_gql(GqlError::Auth(AuthError::NotLoggedIn))?;

		// TODO: Check if the user is allowed to send messages in this chat
		let message_id = Ulid::new();
		let chat_message: database::ChatMessage = scuffle_utils::database::query(
			r#"
			INSERT INTO chat_messages (
				id,
				user_id,
				channel_id,
				content
			) VALUES (
				$1,
				$2,
				$3,
				$4
			) RETURNING *
			"#,
		)
		.bind(message_id)
		.bind(auth.session.user_id)
		.bind(channel_id.to_ulid())
		.bind(content.clone())
		.build_query_as()
		.fetch_one(global.db())
		.await?;

		match global
			.nats()
			.publish(
				SubscriptionTopic::ChannelChatMessages(channel_id.to_ulid()),
				chat_message.to_protobuf().encode_to_vec().into(),
			)
			.await
		{
			Ok(_) => {}
			Err(e) => {
				error!("failed to publish nats message: {}", e);
				return Err(GqlError::InternalServerError("Failed to publish message").into());
			}
		};

		Ok(chat_message.into())
	}
}
