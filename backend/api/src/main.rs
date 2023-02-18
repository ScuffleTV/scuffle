use std::sync::Arc;

use anyhow::Result;
use common::logging;
use tokio::select;

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

    select! {
        _ = api::run(global.clone()) => tracing::info!("api stopped"),
        _ = tokio::signal::ctrl_c() => tracing::info!("ctrl-c received"),
    }

    Ok(())
}
