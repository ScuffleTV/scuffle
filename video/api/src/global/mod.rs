use std::{str::FromStr, sync::Arc, time::Duration};

use anyhow::Result;
use async_nats::ServerAddr;
use common::{context::Context, dataloader::DataLoader};
use sqlx::ConnectOptions;
use sqlx_postgres::PgConnectOptions;

use crate::{config::AppConfig, dataloaders};

pub struct GlobalState {
    pub config: AppConfig,
    pub ctx: Context,
    pub nats: async_nats::Client,
    pub jetstream: async_nats::jetstream::Context,
    pub event_stream: async_nats::jetstream::stream::Stream,
    pub db: Arc<sqlx::PgPool>,

    pub access_token_loader: DataLoader<dataloaders::AccessTokenLoader>,
}

impl GlobalState {
    pub async fn new(ctx: Context, config: AppConfig) -> Result<Self> {
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

        let db = Arc::new(
            sqlx::PgPool::connect_with(
                PgConnectOptions::from_str(&config.database.uri)?
                    .disable_statement_logging()
                    .to_owned(),
            )
            .await?,
        );

        let jetstream = async_nats::jetstream::new(nats.clone());
        let event_stream = jetstream.get_stream(&config.api.event_stream).await?;

        Ok(Self {
            config,
            ctx,
            jetstream: async_nats::jetstream::new(nats.clone()),
            event_stream,
            nats,
            access_token_loader: dataloaders::AccessTokenLoader::new(db.clone()),
            db,
        })
    }
}
