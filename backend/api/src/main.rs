#![cfg_attr(coverage_nightly, feature(no_coverage))]

use std::{str::FromStr, sync::Arc, time::Duration};

use anyhow::Result;
use common::{context::Context, logging, signal};
use fred::{
    clients::SubscriberClient, pool::RedisPool, prelude::ClientLike, types::ReconnectPolicy,
};
use sqlx::{postgres::PgConnectOptions, ConnectOptions};
use tokio::{select, signal::unix::SignalKind, time};

pub mod api;
pub mod config;
pub mod dataloader;
pub mod global;

#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() -> Result<()> {
    let config = config::AppConfig::parse()?;
    logging::init(&config.log_level)?;

    let db = Arc::new(
        sqlx::PgPool::connect_with(
            PgConnectOptions::from_str(&config.database_url)?
                .disable_statement_logging()
                .to_owned(),
        )
        .await?,
    );

    let redis_config = config.get_redis_config();

    let redis_pool = RedisPool::new(redis_config.clone(), 50)?;
    let _ = redis_pool.connect(Some(ReconnectPolicy::default()));
    redis_pool.wait_for_connect().await?;

    let redis_sub_client = SubscriberClient::new(redis_config);
    redis_sub_client.connect(Some(ReconnectPolicy::default()));
    redis_sub_client.wait_for_connect().await.unwrap();
    redis_sub_client.manage_subscriptions();

    let (ctx, handler) = Context::new();

    let global = Arc::new(global::GlobalState::new(
        config,
        db,
        ctx,
        redis_pool,
        redis_sub_client,
    ));

    tracing::info!("starting");

    let api_future = tokio::spawn(api::run(global.clone()));

    // Listen on both sigint and sigterm and cancel the context when either is received
    let mut signal_handler = signal::SignalHandler::new()
        .with_signal(SignalKind::interrupt())
        .with_signal(SignalKind::terminate());

    select! {
        r = api_future => tracing::error!("api stopped unexpectedly: {:?}", r),
        _ = signal_handler.recv() => tracing::info!("shutting down"),
    }

    // We cannot have a context in scope when we cancel the handler, otherwise it will deadlock.
    drop(global);

    // Cancel the context
    tracing::info!("waiting for tasks to finish");

    select! {
        _ = time::sleep(Duration::from_secs(60)) => tracing::warn!("force shutting down"),
        _ = signal_handler.recv() => tracing::warn!("force shutting down"),
        _ = handler.cancel() => tracing::info!("shutting down"),
    }

    Ok(())
}
