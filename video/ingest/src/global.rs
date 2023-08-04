use std::collections::HashMap;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::sync::Arc;

use common::context::Context;
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
    pub fn new(
        mut config: AppConfig,
        db: Arc<sqlx::PgPool>,
        nats: async_nats::Client,
        ctx: Context,
    ) -> Self {
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

        Self {
            config,
            ctx,
            db,
            jetstream: async_nats::jetstream::new(nats.clone()),
            nats,
            requests: Default::default(),
        }
    }
}
