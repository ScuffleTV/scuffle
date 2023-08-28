use std::{str::FromStr, sync::Arc, time::Duration};

use crate::api::v1::gql::schema;
use anyhow::Result;
use async_graphql::SDLExportOptions;
use common::{context::Context, logging, signal};
use fred::types::ReconnectPolicy;
use sqlx::{postgres::PgConnectOptions, ConnectOptions};
use tokio::{select, signal::unix::SignalKind, time};

mod api;
mod config;
mod database;
mod dataloader;
mod global;
// mod grpc;
mod subscription;

// #[cfg(test)]
// mod tests;

#[tokio::main]
async fn main() -> Result<()> {
    let config = config::AppConfig::parse()?;

    if config.export_gql {
        let schema = schema();

        println!(
            "{}",
            schema.sdl_with_options(
                SDLExportOptions::default()
                    .federation()
                    .include_specified_by()
                    .sorted_arguments()
                    .sorted_enum_items()
                    .sorted_fields()
            )
        );

        return Ok(());
    }

    logging::init(&config.logging.level, config.logging.mode)?;

    if let Some(file) = &config.config_file {
        tracing::info!(file = file, "loaded config from file");
    }

    tracing::debug!("config: {:#?}", config);

    let db = Arc::new(
        sqlx::PgPool::connect_with(
            PgConnectOptions::from_str(&config.database.uri)?
                .disable_statement_logging()
                .to_owned(),
        )
        .await?,
    );

    let (ctx, handler) = Context::new();

    // let rmq = common::rmq::ConnectionPool::connect(
    //     config.rmq.uri.clone(),
    //     lapin::ConnectionProperties::default(),
    //     Duration::from_secs(30),
    //     1,
    // )
    // .timeout(Duration::from_secs(5))
    // .await
    // .context("failed to connect to rabbitmq, timedout")?
    // .context("failed to connect to rabbitmq")?;

    let redis = global::setup_redis(&config).await;
    let subscription_redis =
        global::setup_redis_subscription(&config, ReconnectPolicy::new_constant(0, 300)).await;

    tracing::info!("connected to redis");

    let global = Arc::new(global::GlobalState::new(config, db, redis, ctx));

    let api_future = tokio::spawn(api::run(global.clone()));
    // let grpc_future = tokio::spawn(grpc::run(global.clone()));

    // Listen on both sigint and sigterm and cancel the context when either is received
    let mut signal_handler = signal::SignalHandler::new()
        .with_signal(SignalKind::interrupt())
        .with_signal(SignalKind::terminate());

    select! {
        r = api_future => tracing::error!("api stopped unexpectedly: {:?}", r),
        // r = grpc_future => tracing::error!("grpc stopped unexpectedly: {:?}", r),
        r = global.subscription_manager.run(global.ctx.clone(), subscription_redis) => tracing::error!("subscription manager stopped unexpectedly: {:?}", r),
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
