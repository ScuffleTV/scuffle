use crate::global::GlobalState;
use anyhow::Result;
use std::sync::Arc;
use tokio::select;
use tonic::transport::{Certificate, Identity, Server, ServerTlsConfig};

mod access_token;
mod events;
mod playback_key_pair;
mod playback_session;
mod recording;
mod recording_config;
mod room;
mod s3_bucket;
mod transcoding_config;

mod utils;

pub async fn run(global: Arc<GlobalState>) -> Result<()> {
    tracing::info!("API Listening on {}", global.config.api.bind_address);

    let server = if let Some(tls) = &global.config.api.tls {
        let cert = tokio::fs::read(&tls.cert).await?;
        let key = tokio::fs::read(&tls.key).await?;
        let ca_cert = tokio::fs::read(&tls.ca_cert).await?;
        tracing::info!("API TLS enabled");
        Server::builder().tls_config(
            ServerTlsConfig::new()
                .identity(Identity::from_pem(cert, key))
                .client_ca_root(Certificate::from_pem(ca_cert)),
        )?
    } else {
        tracing::info!("API TLS disabled");
        Server::builder()
    }
    .add_service(room::RoomServer::new(&global))
    .add_service(playback_key_pair::PlaybackKeyPairServer::new(&global))
    .add_service(playback_session::PlaybackSessionServer::new(&global))
    .add_service(recording::RecordingServer::new(&global))
    .add_service(recording_config::RecordingConfigServer::new(&global))
    .add_service(transcoding_config::TranscodingConfigServer::new(&global))
    .add_service(s3_bucket::S3BucketServer::new(&global))
    .add_service(access_token::AccessTokenServer::new(&global))
    .add_service(events::EventsServer::new(&global))
    .serve_with_shutdown(global.config.api.bind_address, async {
        global.ctx.done().await;
    });

    select! {
        _ = global.ctx.done() => {
            return Ok(());
        },
        r = server => {
            if let Err(r) = r {
                tracing::error!("API server failed: {:?}", r);
                return Err(r.into());
            }
        },
    }

    Ok(())
}
