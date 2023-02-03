use std::sync::Arc;

use anyhow::Result;
use common::logging;

mod config;
mod transcode;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Arc::new(config::AppConfig::parse()?);

    logging::init(&config.log_level)?;

    tracing::info!("starting");

    transcode::run(config).await?;

    Ok(())
}
