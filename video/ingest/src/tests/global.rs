use std::{sync::Arc, time::Duration};

use common::{
    context::{Context, Handler},
    logging,
    prelude::FutureTimeout,
};

use crate::{config::AppConfig, global::GlobalState};

pub async fn mock_global_state(config: AppConfig) -> (Arc<GlobalState>, Handler) {
    let (ctx, handler) = Context::new();

    dotenvy::dotenv().ok();

    logging::init(&config.logging.level, config.logging.mode)
        .expect("failed to initialize logging");

    let db = Arc::new(
        sqlx::PgPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL not set"))
            .await
            .expect("failed to connect to database"),
    );

    let nats = async_nats::connect(std::env::var("NATS_URL").expect("NATS_URL not set"))
        .timeout(Duration::from_secs(5))
        .await
        .expect("failed to connect to rabbitmq")
        .expect("failed to connect to rabbitmq");

    let global = Arc::new(GlobalState::new(config, db, nats, ctx));

    (global, handler)
}
