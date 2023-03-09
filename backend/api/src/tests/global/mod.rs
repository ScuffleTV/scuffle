use std::sync::Arc;

use crate::{config::AppConfig, global::GlobalState};
use common::{
    context::{Context, Handler},
    logging,
};
use fred::{
    clients::SubscriberClient,
    pool::RedisPool,
    prelude::ClientLike,
    types::{PerformanceConfig, ReconnectPolicy},
};
pub mod turnstile;

pub async fn mock_global_state(config: AppConfig) -> (Arc<GlobalState>, Handler) {
    let (ctx, handler) = Context::new();

    dotenvy::dotenv().ok();

    logging::init("api=debug").expect("failed to initialize logging");

    let db = Arc::new(
        sqlx::PgPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL not set"))
            .await
            .expect("failed to connect to database"),
    );

    let redis_config = config.get_redis_config();
    let redis_pool = RedisPool::new(
        redis_config.clone(),
        Some(PerformanceConfig::default()),
        Some(ReconnectPolicy::default()),
        50,
    )
    .unwrap();
    redis_pool.connect();
    redis_pool.wait_for_connect().await.unwrap();

    let redis_sub_client = SubscriberClient::new(
        redis_config,
        Some(PerformanceConfig::default()),
        Some(ReconnectPolicy::default()),
    );
    redis_sub_client.connect();
    redis_sub_client.wait_for_connect().await.unwrap();
    redis_sub_client.manage_subscriptions();

    (
        Arc::new(GlobalState::new(
            config,
            db,
            ctx,
            redis_pool,
            redis_sub_client,
        )),
        handler,
    )
}
