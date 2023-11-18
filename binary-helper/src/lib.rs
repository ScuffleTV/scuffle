use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context as _;
use common::config::GrpcConfig;
use common::context::Context;
use common::{logging, signal};
use tokio::signal::unix::SignalKind;
use tokio::{select, time};
use tonic::transport::{Certificate, Identity, Server, ServerTlsConfig};
pub use traits::{Config, Global};

pub mod config;
pub mod global;
pub mod grpc_health;
pub mod traits;

pub async fn bootstrap<C: Config, G: Global<C>, F: Future<Output = anyhow::Result<()>> + Send + 'static>(
	process: impl FnOnce(Arc<G>) -> F,
) -> anyhow::Result<()> {
	let (ctx, handler) = Context::new();

	let config = C::parse()
		.and_then(|mut config| {
			config.pre_hook()?;
			Ok(config)
		})
		.map_err(|err| {
			logging::init("trace", Default::default()).expect("failed to init logging");

			err
		})
		.context("failed to parse config")?;

	logging::init(&config.logging().level, config.logging().mode).expect("failed to init logging");

	tracing::info!(name = config.name(), "starting up");

	let global = Arc::new(G::new(ctx, config).await.context("failed to create global state")?);

	tracing::debug!("global state created, starting process");

	let process_future = tokio::spawn(process(global));

	let mut signal_handler = signal::SignalHandler::new()
		.with_signal(SignalKind::interrupt())
		.with_signal(SignalKind::terminate());

	select! {
		_ = signal_handler.recv() => tracing::info!("shutting down"),
		r = process_future => tracing::error!("process stopped unexpectedly: {:#}", match &r {
			Ok(Ok(())) => &"no error raised" as &dyn std::fmt::Display,
			Err(err) => err as &dyn std::fmt::Display,
			Ok(Err(err)) => err as &dyn std::fmt::Display,
		}),
	}

	tracing::info!("waiting for tasks to finish");

	select! {
		_ = time::sleep(Duration::from_secs(60)) => tracing::warn!("force shutting down"),
		_ = signal_handler.recv() => tracing::warn!("force shutting down"),
		_ = handler.cancel() => tracing::info!("shutting down"),
	}

	Ok(())
}

pub async fn grpc_server(config: &GrpcConfig) -> anyhow::Result<tonic::transport::Server> {
	Ok(if let Some(tls) = &config.tls {
		let key = tokio::fs::read(&tls.key).await.context("failed to read grpc private key")?;
		let cert = tokio::fs::read(&tls.cert).await.context("failed to read grpc cert")?;
		let ca_cert = tokio::fs::read(&tls.ca_cert).await.context("failed to read grpc ca cert")?;
		Server::builder().tls_config(
			ServerTlsConfig::new()
				.identity(Identity::from_pem(cert, key))
				.client_ca_root(Certificate::from_pem(ca_cert)),
		)?
	} else {
		Server::builder()
	})
}
