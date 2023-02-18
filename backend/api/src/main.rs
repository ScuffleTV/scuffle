use std::sync::Arc;

use anyhow::Result;
use common::logging;

mod api;
mod config;
mod global;

#[tokio::main]
async fn main() -> Result<()> {
    let config = config::AppConfig::parse()?;
    logging::init(&config.log_level)?;

    let db = sqlx::PgPool::connect(&config.database_url).await?;

    let global = Arc::new(global::GlobalState { config, db });

    tracing::info!("starting");

    api::run(global).await?;

    Ok(())
}
