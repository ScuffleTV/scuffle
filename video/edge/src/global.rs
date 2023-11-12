use crate::{config::EdgeConfig, subscription};

pub trait EdgeState {
    fn metadata_store(&self) -> &async_nats::jetstream::kv::Store;
    fn media_store(&self) -> &async_nats::jetstream::object_store::ObjectStore;
    fn subscriber(&self) -> &subscription::SubscriptionManager;
}

pub trait EdgeGlobal:
    common::global::GlobalCtx
    + common::global::GlobalConfigProvider<EdgeConfig>
    + common::global::GlobalNats
    + common::global::GlobalDb
    + common::global::GlobalConfig
    + EdgeState
    + Send
    + Sync
    + 'static
{
}

impl<T> EdgeGlobal for T where
    T: common::global::GlobalCtx
        + common::global::GlobalConfigProvider<EdgeConfig>
        + common::global::GlobalNats
        + common::global::GlobalDb
        + common::global::GlobalConfig
        + EdgeState
        + Send
        + Sync
        + 'static
{
}
