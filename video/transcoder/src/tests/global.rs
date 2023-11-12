use std::sync::Arc;

use common::{
    context::{Context, Handler},
    grpc::TlsSettings,
    logging,
};

use crate::config::TranscoderConfig;

pub struct GlobalState {
    ctx: Context,
    config: TranscoderConfig,
    nats: async_nats::Client,
    jetstream: async_nats::jetstream::Context,
    db: Arc<sqlx::PgPool>,
    ingest_tls: Option<TlsSettings>,
    media_store: async_nats::jetstream::object_store::ObjectStore,
    metadata_store: async_nats::jetstream::kv::Store,
}

impl common::global::GlobalCtx for GlobalState {
    fn ctx(&self) -> &Context {
        &self.ctx
    }
}

impl common::global::GlobalConfigProvider<TranscoderConfig> for GlobalState {
    fn provide_config(&self) -> &TranscoderConfig {
        &self.config
    }
}

impl common::global::GlobalNats for GlobalState {
    fn nats(&self) -> &async_nats::Client {
        &self.nats
    }

    fn jetstream(&self) -> &async_nats::jetstream::Context {
        &self.jetstream
    }
}

impl common::global::GlobalDb for GlobalState {
    fn db(&self) -> &Arc<sqlx::PgPool> {
        &self.db
    }
}

impl common::global::GlobalConfig for GlobalState {}

impl crate::global::TranscoderState for GlobalState {
    fn ingest_tls(&self) -> Option<common::grpc::TlsSettings> {
        self.ingest_tls.clone()
    }

    fn media_store(&self) -> &async_nats::jetstream::object_store::ObjectStore {
        &self.media_store
    }

    fn metadata_store(&self) -> &async_nats::jetstream::kv::Store {
        &self.metadata_store
    }
}

pub async fn mock_global_state(config: TranscoderConfig) -> (Arc<GlobalState>, Handler) {
    let (ctx, handler) = Context::new();

    dotenvy::dotenv().ok();

    let logging_level = std::env::var("LOGGING_LEVEL").unwrap_or_else(|_| "info".to_string());

    logging::init(&logging_level, Default::default()).expect("failed to initialize logging");

    let database_uri = std::env::var("VIDEO_DATABASE_URL").expect("DATABASE_URL must be set");
    let nats_addr = std::env::var("NATS_ADDR").expect("NATS_URL must be set");

    let nats = async_nats::connect(&nats_addr)
        .await
        .expect("failed to connect to nats");
    let jetstream = async_nats::jetstream::new(nats.clone());

    let db = Arc::new(
        sqlx::PgPool::connect(&database_uri)
            .await
            .expect("failed to connect to database"),
    );

    let metadata_store = jetstream
        .create_key_value(async_nats::jetstream::kv::Config {
            bucket: config.metadata_kv_store.clone(),
            ..Default::default()
        })
        .await
        .unwrap();

    let media_store = jetstream
        .create_object_store(async_nats::jetstream::object_store::Config {
            bucket: config.media_ob_store.clone(),
            ..Default::default()
        })
        .await
        .unwrap();

    jetstream
        .create_stream(async_nats::jetstream::stream::Config {
            name: config.transcoder_request_subject.clone(),
            ..Default::default()
        })
        .await
        .unwrap();

    let global = Arc::new(GlobalState {
        ingest_tls: config.ingest_tls.as_ref().map(|tls| {
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
        nats,
        jetstream,
        db,
        media_store,
        metadata_store,
    });

    (global, handler)
}
