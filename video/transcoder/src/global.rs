use crate::config::TranscoderConfig;
use common::grpc::TlsSettings;

pub trait TranscoderState {
    fn metadata_store(&self) -> &async_nats::jetstream::kv::Store;
    fn media_store(&self) -> &async_nats::jetstream::object_store::ObjectStore;
    fn ingest_tls(&self) -> Option<TlsSettings>;
}

pub trait TranscoderGlobal:
    common::global::GlobalCtx
    + common::global::GlobalConfigProvider<TranscoderConfig>
    + common::global::GlobalNats
    + common::global::GlobalDb
    + common::global::GlobalConfig
    + TranscoderState
    + Send
    + Sync
    + 'static
{
}

impl<T> TranscoderGlobal for T where
    T: common::global::GlobalCtx
        + common::global::GlobalConfigProvider<TranscoderConfig>
        + common::global::GlobalNats
        + common::global::GlobalDb
        + common::global::GlobalConfig
        + TranscoderState
        + Send
        + Sync
        + 'static
{
}
