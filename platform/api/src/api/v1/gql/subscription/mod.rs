use async_graphql::{MergedSubscription, Subscription};
use futures_util::Stream;

use self::{chat::ChatSubscription, user::UserSubscription};

pub mod chat;
pub mod user;

#[derive(MergedSubscription, Default)]
pub struct Subscription(UserSubscription, ChatSubscription, NoopSubscription);

#[derive(Default)]
struct NoopSubscription;

#[Subscription]
impl NoopSubscription {
    async fn noop(&self) -> impl Stream<Item = bool> {
        futures_util::stream::empty()
    }
}
