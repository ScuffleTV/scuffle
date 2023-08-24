use crate::config::AppConfig;
use std::sync::Arc;
use std::time::Duration;

use async_graphql::dataloader::DataLoader;
use common::context::Context;
use common::prelude::FutureTimeout;
use fred::clients::SubscriberClient;
use fred::native_tls;
use fred::pool::RedisPool;
use fred::prelude::ClientLike;
use fred::types::{ReconnectPolicy, RedisConfig, ServerConfig};

use crate::dataloader::stream::{ActiveStreamsByUserIdLoader, StreamByIdLoader};
use crate::dataloader::user_permissions::UserPermissionsByIdLoader;
use crate::dataloader::{
    session::SessionByIdLoader, user::UserByIdLoader, user::UserByUsernameLoader,
};
use crate::subscription::SubscriptionManager;

pub mod turnstile;

pub struct GlobalState {
    pub config: AppConfig,
    pub db: Arc<sqlx::PgPool>,
    pub ctx: Context,
    pub user_by_username_loader: DataLoader<UserByUsernameLoader>,
    pub user_by_id_loader: DataLoader<UserByIdLoader>,
    pub session_by_id_loader: DataLoader<SessionByIdLoader>,
    pub user_permisions_by_id_loader: DataLoader<UserPermissionsByIdLoader>,
    pub stream_by_id_loader: DataLoader<StreamByIdLoader>,
    pub active_streams_by_user_id_loader: DataLoader<ActiveStreamsByUserIdLoader>,
    pub subscription_manager: SubscriptionManager,
    pub redis: RedisPool,
}

impl GlobalState {
    pub fn new(config: AppConfig, db: Arc<sqlx::PgPool>, redis: RedisPool, ctx: Context) -> Self {
        Self {
            config,
            ctx,
            user_by_username_loader: UserByUsernameLoader::new(db.clone()),
            user_by_id_loader: UserByIdLoader::new(db.clone()),
            session_by_id_loader: SessionByIdLoader::new(db.clone()),
            user_permisions_by_id_loader: UserPermissionsByIdLoader::new(db.clone()),
            stream_by_id_loader: StreamByIdLoader::new(db.clone()),
            active_streams_by_user_id_loader: ActiveStreamsByUserIdLoader::new(db.clone()),
            subscription_manager: SubscriptionManager::default(),
            db,
            redis,
        }
    }
}

pub fn redis_config(config: &AppConfig) -> RedisConfig {
    RedisConfig {
        database: Some(config.redis.database),
        username: config.redis.username.clone(),
        password: config.redis.password.clone(),
        server: if let Some(sentinel) = &config.redis.sentinel {
            let addresses = config
                .redis
                .addresses
                .iter()
                .map(|a| {
                    let mut parts = a.split(':');
                    let host = parts.next().expect("no redis host");
                    let port = parts
                        .next()
                        .expect("no redis port")
                        .parse()
                        .expect("failed to parse redis port");

                    (host, port)
                })
                .collect::<Vec<_>>();

            ServerConfig::new_sentinel(addresses, sentinel.service_name.clone())
        } else {
            let server = config.redis.addresses.first().expect("no redis addresses");
            if config.redis.addresses.len() > 1 {
                tracing::warn!("multiple redis addresses, only using first: {}", server);
            }

            let mut parts = server.split(':');
            let host = parts.next().expect("no redis host");
            let port = parts
                .next()
                .expect("no redis port")
                .parse()
                .expect("failed to parse redis port");

            ServerConfig::new_centralized(host, port)
        },
        tls: if let Some(tls) = &config.redis.tls {
            let cert = std::fs::read(&tls.cert).expect("failed to read redis cert");
            let key = std::fs::read(&tls.key).expect("failed to read redis key");
            let ca_cert = std::fs::read(&tls.ca_cert).expect("failed to read redis ca");

            Some(
                fred::native_tls::TlsConnector::builder()
                    .identity(
                        native_tls::Identity::from_pkcs8(&cert, &key)
                            .expect("failed to parse redis cert/key"),
                    )
                    .add_root_certificate(
                        native_tls::Certificate::from_pem(&ca_cert)
                            .expect("failed to parse redis ca"),
                    )
                    .build()
                    .expect("failed to build redis tls")
                    .into(),
            )
        } else {
            None
        },
        ..Default::default()
    }
}

pub async fn setup_redis(config: &AppConfig) -> RedisPool {
    let redis = RedisPool::new(
        redis_config(config),
        Some(Default::default()),
        Some(Default::default()),
        config.redis.pool_size,
    )
    .expect("failed to create redis pool");

    redis.connect();

    redis
        .wait_for_connect()
        .timeout(Duration::from_secs(2))
        .await
        .expect("failed to connect to redis")
        .expect("failed to connect to redis");

    redis
}

pub async fn setup_redis_subscription(
    config: &AppConfig,
    reconnect_policy: ReconnectPolicy,
) -> SubscriberClient {
    let redis = SubscriberClient::new(
        redis_config(config),
        Some(Default::default()),
        Some(reconnect_policy),
    );

    redis.connect();

    redis
        .wait_for_connect()
        .timeout(Duration::from_secs(2))
        .await
        .expect("failed to connect to redis")
        .expect("failed to connect to redis");

    redis
}
