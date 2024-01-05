use async_graphql::SimpleObject;

use crate::global::ApiGlobal;

mod auth;
mod channel;
mod chat;
mod user;

#[derive(SimpleObject)]
/// The root mutation type which contains root level fields.
pub struct Mutation<G: ApiGlobal> {
	auth: auth::AuthMutation<G>,
	chat: chat::ChatMutation<G>,
	user: user::UserMutation<G>,
	channel: channel::ChannelMutation<G>,
}

impl<G: ApiGlobal> Default for Mutation<G> {
	fn default() -> Self {
		Self {
			auth: Default::default(),
			chat: Default::default(),
			user: Default::default(),
			channel: Default::default(),
		}
	}
}
