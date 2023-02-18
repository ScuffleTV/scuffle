use std::sync::Arc;

use anyhow::Result;
use common::logging;
use tokio::select;

mod config;
mod transcoder;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Arc::new(config::AppConfig::parse()?);

    logging::init(&config.log_level)?;

    tracing::info!("starting");

    select! {
        _ = transcoder::run(config.clone()) => tracing::info!("transcoder stopped"),
        _ = tokio::signal::ctrl_c() => tracing::info!("ctrl-c received"),
    }

    Ok(())
}
