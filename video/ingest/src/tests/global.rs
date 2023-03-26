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

    logging::init(&config.logging.level, config.logging.json)
        .expect("failed to initialize logging");

    let rmq = common::rmq::ConnectionPool::connect(
        std::env::var("RMQ_URL").expect("RMQ_URL not set"),
        lapin::ConnectionProperties::default(),
        Duration::from_secs(30),
        1,
    )
    .timeout(Duration::from_secs(5))
    .await
    .expect("failed to connect to rabbitmq")
    .expect("failed to connect to rabbitmq");

    let global = Arc::new(GlobalState::new(config, ctx, rmq));

    (global, handler)
}
