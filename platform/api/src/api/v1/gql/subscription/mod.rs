use async_graphql::{MergedSubscription, SimpleObject, Subscription};
use futures_util::Stream;

use self::{channel::ChannelSubscription, chat::ChatSubscription, user::UserSubscription};

use super::models::ulid::GqlUlid;

mod channel;
mod chat;
mod user;

#[derive(SimpleObject)]
struct FollowStream {
    pub user_id: GqlUlid,
    pub channel_id: GqlUlid,
    pub following: bool,
}

#[derive(MergedSubscription, Default)]
pub struct Subscription(
    UserSubscription,
    ChannelSubscription,
    ChatSubscription,
    NoopSubscription,
);

#[derive(Default)]
struct NoopSubscription;

#[Subscription]
impl NoopSubscription {
    async fn noop(&self) -> impl Stream<Item = bool> {
        futures_util::stream::empty()
    }
}
