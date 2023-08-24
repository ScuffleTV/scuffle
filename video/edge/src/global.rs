use std::sync::Arc;

use common::context::Context;

use crate::config::AppConfig;

pub struct GlobalState {
    pub config: AppConfig,
    pub ctx: Context,
    pub nats: async_nats::Client,
    pub jetstream: async_nats::jetstream::Context,
    pub metadata_store: async_nats::jetstream::kv::Store,
    pub media_store: async_nats::jetstream::object_store::ObjectStore,
    pub db: Arc<sqlx::PgPool>,
}

impl GlobalState {
    pub fn new(
        config: AppConfig,
        ctx: Context,
        nats: async_nats::Client,
        db: Arc<sqlx::PgPool>,
        metadata_store: async_nats::jetstream::kv::Store,
        media_store: async_nats::jetstream::object_store::ObjectStore,
    ) -> Self {
        Self {
            config,
            ctx,
            jetstream: async_nats::jetstream::new(nats.clone()),
            nats,
            metadata_store,
            media_store,
            db,
        }
    }
}
