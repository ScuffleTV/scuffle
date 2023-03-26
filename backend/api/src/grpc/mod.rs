use crate::global::GlobalState;
use anyhow::Result;
use std::sync::Arc;
use tokio::select;
use tonic::transport::{Certificate, Identity, Server, ServerTlsConfig};

pub mod api;
pub mod health;
pub mod pb;

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
    .add_service(api::ApiServer::new(&global).into_service())
    .add_service(health::HealthServer::new(&global).into_service())
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
