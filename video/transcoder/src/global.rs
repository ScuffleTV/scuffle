use std::sync::Arc;

use common::{context::Context, grpc::TlsSettings};

use crate::config::AppConfig;

pub struct GlobalState {
    pub config: AppConfig,
    pub ctx: Context,
    pub nats: async_nats::Client,
    pub jetstream: async_nats::jetstream::Context,
    pub media_store: async_nats::jetstream::object_store::ObjectStore,
    pub metadata_store: async_nats::jetstream::kv::Store,
    pub db: Arc<sqlx::PgPool>,
    ingest_tls: Option<TlsSettings>,
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
            ingest_tls: config.transcoder.ingest_tls.as_ref().map(|tls| {
                let cert = std::fs::read(&tls.cert).expect("failed to read redis cert");
                let key = std::fs::read(&tls.key).expect("failed to read redis key");
                let ca_cert = std::fs::read(&tls.ca_cert).expect("failed to read redis ca");

                TlsSettings {
                    domain: tls.domain.clone(),
                    ca_cert: tonic::transport::Certificate::from_pem(ca_cert),
                    identity: tonic::transport::Identity::from_pem(cert, key),
                }
            }),
            config,
            ctx,
            jetstream: async_nats::jetstream::new(nats.clone()),
            nats,
            db,
            metadata_store,
            media_store,
        }
    }

    pub fn ingest_tls(&self) -> Option<TlsSettings> {
        self.ingest_tls.clone()
    }
}
