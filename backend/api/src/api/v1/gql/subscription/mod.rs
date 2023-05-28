use async_graphql::{MergedSubscription, Subscription};
use futures_util::Stream;

use self::user::UserSubscription;

pub mod user;

#[derive(MergedSubscription, Default)]
pub struct Subscription(UserSubscription, NoopSubscription);

#[derive(Default)]
struct NoopSubscription;

#[Subscription]
impl NoopSubscription {
    async fn noop(&self) -> impl Stream<Item = bool> {
        futures_util::stream::empty()
    }
}
