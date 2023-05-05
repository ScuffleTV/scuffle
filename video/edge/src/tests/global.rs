use std::sync::Arc;

use common::{
    context::{Context, Handler},
    logging,
};
use fred::pool::RedisPool;

use crate::{config::AppConfig, global::GlobalState};

pub async fn mock_global_state(config: AppConfig) -> (Arc<GlobalState>, Handler) {
    let (ctx, handler) = Context::new();

    dotenvy::dotenv().ok();

    logging::init(&config.logging.level, config.logging.json)
        .expect("failed to initialize logging");

    let redis = RedisPool::new(
        fred::types::RedisConfig::from_url(
            std::env::var("REDIS_URL").expect("REDIS_URL not set").as_str(),
        ).expect("failed to parse redis url"),
        Some(Default::default()),
        Some(Default::default()),
        2,
    )
    .expect("failed to create redis pool");

    let global = Arc::new(GlobalState::new(config, ctx, redis));

    (global, handler)
}
