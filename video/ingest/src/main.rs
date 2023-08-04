use std::{str::FromStr, sync::Arc, time::Duration};

use anyhow::Result;
use async_nats::ServerAddr;
use common::{context::Context, logging, signal};
use sqlx::ConnectOptions;
use sqlx_postgres::PgConnectOptions;
use tokio::{select, signal::unix::SignalKind, time};

mod config;
mod define;
mod global;
mod grpc;
mod ingest;

#[tokio::main]
async fn main() -> Result<()> {
    let config = config::AppConfig::parse()?;

    logging::init(&config.logging.level, config.logging.mode)?;

    if let Some(file) = &config.config_file {
        tracing::info!(file = file, "loaded config from file");
    }

    let (ctx, handler) = Context::new();

    let db = Arc::new(
        sqlx::PgPool::connect_with(
            PgConnectOptions::from_str(&config.database.uri)?
                .disable_statement_logging()
                .to_owned(),
        )
        .await?,
    );

    let nats = {
        let mut options = async_nats::ConnectOptions::new()
            .connection_timeout(Duration::from_secs(5))
            .name(&config.name)
            .retry_on_initial_connect();

        if let Some(user) = &config.nats.username {
            options = options.user_and_password(
                user.clone(),
                config.nats.password.clone().unwrap_or_default(),
            )
        } else if let Some(token) = &config.nats.token {
            options = options.token(token.clone())
        }

        if let Some(tls) = &config.nats.tls {
            options = options
                .require_tls(true)
                .add_root_certificates((&tls.ca_cert).into())
                .add_client_certificate((&tls.cert).into(), (&tls.key).into());
        }

        options
            .connect(
                config
                    .nats
                    .servers
                    .iter()
                    .map(|s| s.parse::<ServerAddr>())
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .await?
    };

    let global = Arc::new(global::GlobalState::new(config, db, nats, ctx));

    let ingest_future = tokio::spawn(ingest::run(global.clone()));
    let grpc_future = tokio::spawn(grpc::run(global.clone()));

    // Listen on both sigint and sigterm and cancel the context when either is received
    let mut signal_handler = signal::SignalHandler::new()
        .with_signal(SignalKind::interrupt())
        .with_signal(SignalKind::terminate());

    select! {
        r = ingest_future => tracing::error!("api stopped unexpectedly: {:?}", r),
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

#[cfg(test)]
mod tests;
