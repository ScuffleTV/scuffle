use std::{str::FromStr, sync::Arc, time::Duration};

use anyhow::Result;
use async_nats::ServerAddr;
use common::{context::Context, grpc::TlsSettings};
use sqlx::ConnectOptions;
use sqlx_postgres::PgConnectOptions;

use crate::config::AppConfig;

pub struct GlobalState {
    pub config: AppConfig,
    pub ctx: Context,
    pub nats: async_nats::Client,
    pub jetstream: async_nats::jetstream::Context,
    pub media_store: async_nats::jetstream::object_store::ObjectStore,
    pub metadata_store: async_nats::jetstream::kv::Store,
    pub db: Arc<sqlx::PgPool>,
    ingest_tls: Option<TlsSettings>,
}

pub async fn connect_to_nats(config: &AppConfig) -> Result<async_nats::Client> {
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

    Ok(options
        .connect(
            config
                .nats
                .servers
                .iter()
                .map(|s| s.parse::<ServerAddr>())
                .collect::<Result<Vec<_>, _>>()?,
        )
        .await?)
}

impl GlobalState {
    pub async fn new(ctx: Context, config: AppConfig) -> Result<Self> {
        let nats = connect_to_nats(&config).await?;

        let db = Arc::new(
            sqlx::PgPool::connect_with(
                PgConnectOptions::from_str(&config.database.uri)?
                    .disable_statement_logging()
                    .to_owned(),
            )
            .await?,
        );

        let jetstream = async_nats::jetstream::new(nats.clone());

        let metadata_store = jetstream
            .get_key_value(config.transcoder.metadata_kv_store.clone())
            .await?;
        let media_store = jetstream
            .get_object_store(config.transcoder.media_ob_store.clone())
            .await?;

        Ok(Self {
            ingest_tls: config.transcoder.ingest_tls.as_ref().map(|tls| {
                let cert = std::fs::read(&tls.cert).expect("failed to read redis cert");
                let key = std::fs::read(&tls.key).expect("failed to read redis key");
                let ca_cert = std::fs::read(&tls.ca_cert).expect("failed to read redis ca");

                TlsSettings {
                    domain: tls.domain.clone(),
                    ca_cert: tonic::transport::Certificate::from_pem(ca_cert),
                    identity: tonic::transport::Identity::from_pem(cert, key),
                }
            }),
            config,
            ctx,
            jetstream: async_nats::jetstream::new(nats.clone()),
            nats,
            db,
            metadata_store,
            media_store,
        })
    }

    pub fn ingest_tls(&self) -> Option<TlsSettings> {
        self.ingest_tls.clone()
    }
}
