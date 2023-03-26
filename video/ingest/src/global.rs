use std::net::IpAddr;
use std::{net::SocketAddr, time::Duration};

use common::{
    context::Context,
    grpc::{make_channel, TlsSettings},
};
use tonic::transport::{Certificate, Channel, Identity};

use crate::{
    config::AppConfig, connection_manager::StreamManager,
    pb::scuffle::backend::api_client::ApiClient,
};

pub struct GlobalState {
    pub config: AppConfig,
    pub ctx: Context,
    pub rmq: common::rmq::ConnectionPool,
    pub connection_manager: StreamManager,
    api_client: ApiClient<Channel>,
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
    pub fn new(mut config: AppConfig, ctx: Context, rmq: common::rmq::ConnectionPool) -> Self {
        let api_channel = make_channel(
            config.api.addresses.clone(),
            Duration::from_secs(config.api.resolve_interval),
            if let Some(tls) = &config.api.tls {
                let cert = std::fs::read(&tls.cert).expect("failed to read api cert");
                let key = std::fs::read(&tls.key).expect("failed to read api key");
                let ca = std::fs::read(&tls.ca_cert).expect("failed to read api ca");

                let ca_cert = Certificate::from_pem(ca);
                let identity = Identity::from_pem(cert, key);

                Some(TlsSettings {
                    ca_cert,
                    identity,
                    domain: tls.domain.clone().unwrap_or_default(),
                })
            } else {
                None
            },
        )
        .expect("failed to create api channel");

        let api_client = ApiClient::new(api_channel);

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
            api_client,
            rmq,
            connection_manager: StreamManager::new(),
        }
    }

    pub fn api_client(&self) -> ApiClient<Channel> {
        self.api_client.clone()
    }
}
