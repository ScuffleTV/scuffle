use crate::global::GlobalState;
use anyhow::Result;
use std::sync::Arc;
use tokio::select;
use tonic::transport::{Certificate, Identity, Server, ServerTlsConfig};

mod events;
mod health;
mod playback_key_pair;
mod playback_session;
mod recording;
mod recording_config;
mod room;
mod transcoder_config;

mod utils;

pub async fn run(global: Arc<GlobalState>) -> Result<()> {
    tracing::info!("GRPC Listening on {}", global.config.grpc.bind_address);

    let server = if let Some(tls) = &global.config.grpc.tls {
        let cert = tokio::fs::read(&tls.cert).await?;
        let key = tokio::fs::read(&tls.key).await?;
        let ca_cert = tokio::fs::read(&tls.ca_cert).await?;
        tracing::info!("gRPC TLS enabled");
        Server::builder().tls_config(
            ServerTlsConfig::new()
                .identity(Identity::from_pem(cert, key))
                .client_ca_root(Certificate::from_pem(ca_cert)),
        )?
    } else {
        tracing::info!("gRPC TLS disabled");
        Server::builder()
    }
    .add_service(room::RoomServer::new(&global))
    .add_service(health::HealthServer::new(&global))
    .add_service(playback_key_pair::PlaybackKeyPairServer::new(&global))
    .add_service(playback_session::PlaybackSessionServer::new(&global))
    .add_service(recording::RecordingServer::new(&global))
    .add_service(recording_config::RecordingConfigServer::new(&global))
    .add_service(transcoder_config::TranscoderConfigServer::new(&global))
    .add_service(events::EventsServer::new(&global))
    .serve_with_shutdown(global.config.grpc.bind_address, async {
        global.ctx.done().await;
    });

    select! {
        _ = global.ctx.done() => {
            return Ok(());
        },
        r = server => {
            if let Err(r) = r {
                tracing::error!("gRPC server failed: {:?}", r);
                return Err(r.into());
            }
        },
    }

    Ok(())
}
