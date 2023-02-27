use std::sync::Arc;

use crate::{config::AppConfig, global::GlobalState};
use common::{
    context::{Context, Handler},
    logging,
};
use fred::{
    clients::SubscriberClient,
    pool::RedisPool,
    prelude::{ClientLike, ReconnectPolicy, RedisConfig, ServerConfig},
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

    let redis_config = if config.redis_sentinel {
        RedisConfig {
            server: ServerConfig::Sentinel {
                hosts: config.redis_urls.to_vec(),
                service_name: "scuffle-redis".to_string(),
                #[cfg(feature = "sentinel-auth")]
                username: Some(config.redis_username.clone()),
                #[cfg(feature = "sentinel-auth")]
                password: Some(config.redis_password.clone()),
            },
            ..Default::default()
        }
    } else {
        let (host, port) = &config.redis_urls[0];
        RedisConfig {
            server: ServerConfig::new_centralized(host.clone(), *port),
            ..Default::default()
        }
    };

    let redis_pool = RedisPool::new(redis_config.clone(), 2).unwrap();
    let _ = redis_pool.connect(Some(ReconnectPolicy::default()));
    let _ = redis_pool.wait_for_connect().await;

    let redis_sub_client = SubscriberClient::new(redis_config);
    redis_sub_client.connect(Some(ReconnectPolicy::default()));
    redis_sub_client.wait_for_connect().await.unwrap();
    redis_sub_client.manage_subscriptions();

    (Arc::new(GlobalState::new(config, db, ctx, redis_pool, redis_sub_client,)), handler)
}
