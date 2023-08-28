use std::sync::Arc;

use common::{
    context::{Context, Handler},
    logging,
};

use crate::{config::AppConfig, global::GlobalState};

pub async fn mock_global_state(mut config: AppConfig) -> (Arc<GlobalState>, Handler) {
    let (ctx, handler) = Context::new();

    dotenvy::dotenv().ok();

    logging::init(&config.logging.level, config.logging.mode)
        .expect("failed to initialize logging");

    config.database.uri = std::env::var("DATABASE_URI").expect("DATABASE_URL must be set");
    config.nats.servers = vec![std::env::var("NATS_ADDR").expect("NATS_URL must be set")];

    let global = Arc::new(GlobalState::new(ctx, config).await.unwrap());

    (global, handler)
}
