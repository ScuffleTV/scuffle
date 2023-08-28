use std::sync::Arc;

use common::{
    context::{Context, Handler},
    logging,
};

use crate::{
    config::AppConfig,
    global::{connect_to_nats, GlobalState},
};

pub async fn mock_global_state(mut config: AppConfig) -> (Arc<GlobalState>, Handler) {
    let (ctx, handler) = Context::new();

    dotenvy::dotenv().ok();

    logging::init(&config.logging.level, config.logging.mode)
        .expect("failed to initialize logging");

    config.database.uri = std::env::var("DATABASE_URI").expect("DATABASE_URL must be set");
    config.nats.servers = vec![std::env::var("NATS_ADDR").expect("NATS_URL must be set")];

    {
        let nats = connect_to_nats(&config).await.unwrap();

        let jetstream = async_nats::jetstream::new(nats.clone());

        jetstream
            .create_key_value(async_nats::jetstream::kv::Config {
                bucket: config.transcoder.metadata_kv_store.clone(),
                ..Default::default()
            })
            .await
            .unwrap();

        jetstream
            .create_object_store(async_nats::jetstream::object_store::Config {
                bucket: config.transcoder.media_ob_store.clone(),
                ..Default::default()
            })
            .await
            .unwrap();

        jetstream
            .create_stream(async_nats::jetstream::stream::Config {
                name: config.transcoder.transcoder_request_subject.clone(),
                ..Default::default()
            })
            .await
            .unwrap();
    }

    let global = Arc::new(GlobalState::new(ctx, config).await.unwrap());

    (global, handler)
}
