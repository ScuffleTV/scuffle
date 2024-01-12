use async_graphql::{MergedSubscription, SimpleObject, Subscription};
use futures_util::Stream;

use self::channel::ChannelSubscription;
use self::chat::ChatSubscription;
use self::file::FileSubscription;
use self::user::UserSubscription;
use super::models::ulid::GqlUlid;
use crate::global::ApiGlobal;

mod channel;
mod chat;
mod file;
mod user;

#[derive(SimpleObject)]
struct FollowStream {
	pub user_id: GqlUlid,
	pub channel_id: GqlUlid,
	pub following: bool,
}

#[derive(MergedSubscription)]
pub struct Subscription<G: ApiGlobal>(
	ChannelSubscription<G>,
	ChatSubscription<G>,
	FileSubscription<G>,
	UserSubscription<G>,
	NoopSubscription,
);

impl<G: ApiGlobal> Default for Subscription<G> {
	fn default() -> Self {
		Self(Default::default(), Default::default(), Default::default(), Default::default(), Default::default())
	}
}

#[derive(Default)]
struct NoopSubscription;

#[Subscription]
impl NoopSubscription {
	async fn noop(&self) -> impl Stream<Item = bool> {
		futures_util::stream::empty()
	}
}
