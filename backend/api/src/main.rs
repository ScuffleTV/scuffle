use std::sync::Arc;

use anyhow::Result;
use common::logging;

mod api;
mod config;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Arc::new(config::AppConfig::parse()?);

    logging::init(&config.log_level)?;

    tracing::info!("starting");

    api::run(config).await?;

    Ok(())
}
