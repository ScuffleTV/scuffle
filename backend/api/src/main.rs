use std::{sync::Arc, time::Duration};

use anyhow::Result;
use common::{context::Context, logging, signal};
use tokio::{select, signal::unix::SignalKind, time};

mod api;
mod config;
mod global;

#[tokio::main]
async fn main() -> Result<()> {
    let config = config::AppConfig::parse()?;
    logging::init(&config.log_level)?;

    let db = sqlx::PgPool::connect(&config.database_url).await?;

    let (ctx, handler) = Context::new();

    let global = Arc::new(global::GlobalState { config, db, ctx });

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
