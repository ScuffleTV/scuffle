use std::collections::HashMap;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_nats::ServerAddr;
use common::context::Context;
use sqlx::ConnectOptions;
use sqlx_postgres::PgConnectOptions;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use ulid::Ulid;

use crate::config::AppConfig;
use crate::define::IncomingTranscoder;

pub struct GlobalState {
    pub config: AppConfig,
    pub ctx: Context,
    pub db: Arc<sqlx::PgPool>,
    pub nats: async_nats::Client,
    pub jetstream: async_nats::jetstream::Context,
    pub requests: Mutex<HashMap<Ulid, mpsc::Sender<IncomingTranscoder>>>,
}

fn get_local_ip() -> IpAddr {
    let interfaces = pnet::datalink::interfaces();

    let ips = interfaces
        .iter()
        .filter(|i| i.is_up() && !i.ips.is_empty() && !i.is_loopback())
        .flat_map(|i| i.ips.clone())
        .map(|ip| ip.ip())
        .filter(|ip| !ip.is_loopback() && ip.is_ipv4())
        .collect::<Vec<_>>();

    if ips.len() > 1 {
        tracing::info!("multiple ips found, using first one");
    }

    ips[0]
}

impl GlobalState {
    pub async fn new(ctx: Context, mut config: AppConfig) -> Result<Self> {
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

        if config.grpc.advertise_address.is_empty() {
            // We need to figure out what our advertise address is
            let port = config.grpc.bind_address.port();
            let mut advertise_address = config.grpc.bind_address.ip();
            // If the bind_address is [::] or 0.0.0.0 we need to figure out what our
            // actual IP address is.
            if advertise_address.is_unspecified() {
                advertise_address = get_local_ip();
            }

            config.grpc.advertise_address = SocketAddr::new(advertise_address, port).to_string();
        }

        Ok(Self {
            config,
            ctx,
            db,
            jetstream: async_nats::jetstream::new(nats.clone()),
            nats,
            requests: Default::default(),
        })
    }
}
