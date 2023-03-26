use std::sync::Arc;

use common::context::Context;
use fred::{
    pool::RedisPool,
    types::{PerformanceConfig, ReconnectPolicy, RedisConfig, ServerConfig},
};
use lapin::{
    options::QueueDeclareOptions,
    types::{AMQPValue, FieldTable},
};

use crate::config::AppConfig;

pub struct GlobalState {
    pub config: AppConfig,
    pub ctx: Context,
    pub rmq: common::rmq::ConnectionPool,
    pub redis: RedisPool,
}

impl GlobalState {
    pub fn new(
        config: AppConfig,
        ctx: Context,
        rmq: common::rmq::ConnectionPool,
        redis: RedisPool,
    ) -> Self {
        Self {
            config,
            ctx,
            rmq,
            redis,
        }
    }
}

pub async fn init_rmq(global: &Arc<GlobalState>) {
    let channel = global.rmq.aquire().await.expect("failed to create channel");

    let mut options = FieldTable::default();

    options.insert("x-message-ttl".into(), AMQPValue::LongUInt(60 * 1000));

    channel
        .queue_declare(
            &global.config.rmq.transcoder_queue,
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            options,
        )
        .await
        .expect("failed to declare queue");
}

pub fn setup_redis(config: &AppConfig) -> RedisPool {
    let mut redis_config = RedisConfig::default();
    let performance = PerformanceConfig::default();
    let policy = ReconnectPolicy::default();

    redis_config.database = Some(config.redis.database);
    redis_config.username = config.redis.username.clone();
    redis_config.password = config.redis.password.clone();

    redis_config.server = if let Some(sentinel) = &config.redis.sentinel {
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
    };

    redis_config.tls = if let Some(tls) = &config.redis.tls {
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
                    native_tls::Certificate::from_pem(&ca_cert).expect("failed to parse redis ca"),
                )
                .build()
                .expect("failed to build redis tls")
                .into(),
        )
    } else {
        None
    };

    RedisPool::new(
        redis_config,
        Some(performance),
        Some(policy),
        config.redis.pool_size,
    )
    .expect("failed to create redis pool")
}
