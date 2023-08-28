use std::{sync::Arc, time::Duration};

use anyhow::Result;
use common::{context::Context, logging, signal};
use tokio::{select, signal::unix::SignalKind, time};

mod api;
mod config;
mod dataloaders;
mod global;
mod grpc;

#[tokio::main]
async fn main() -> Result<()> {
    let config = config::AppConfig::parse()?;

    logging::init(&config.logging.level, config.logging.mode)?;

    if let Some(file) = &config.config_file {
        tracing::info!(file = file, "loaded config from file");
    }

    let (ctx, handler) = Context::new();

    let global = Arc::new(global::GlobalState::new(ctx, config).await?);

    let grpc_future = tokio::spawn(grpc::run(global.clone()));
    let api_future = tokio::spawn(api::run(global.clone()));

    // Listen on both sigint and sigterm and cancel the context when either is received
    let mut signal_handler = signal::SignalHandler::new()
        .with_signal(SignalKind::interrupt())
        .with_signal(SignalKind::terminate());

    select! {
        r = api_future => tracing::error!("api stopped unexpectedly: {:?}", r),
        r = grpc_future => tracing::error!("grpc stopped unexpectedly: {:?}", r),
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
