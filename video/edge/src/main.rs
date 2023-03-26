use std::sync::Arc;

use anyhow::Result;
use common::logging;
use tokio::select;

mod config;
mod edge;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Arc::new(config::AppConfig::parse()?);

    logging::init(&config.log_level, false)?;

    tracing::info!("starting");

    select! {
        _ = edge::run(config.clone()) => tracing::info!("edge stopped"),
        _ = tokio::signal::ctrl_c() => tracing::info!("ctrl-c received"),
    }

    Ok(())
}
