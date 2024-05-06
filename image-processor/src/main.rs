use std::sync::Arc;

use anyhow::Context;
use scuffle_foundations::bootstrap::{bootstrap, Bootstrap};
use scuffle_foundations::runtime;
use scuffle_foundations::settings::cli::Matches;
use scuffle_image_processor_proto::{event_callback, EventCallback};
use tokio::signal::unix::SignalKind;

use self::config::ImageProcessorConfig;

impl Bootstrap for ImageProcessorConfig {
	type Settings = Self;

	fn runtime_mode(&self) -> scuffle_foundations::bootstrap::RuntimeSettings {
		self.runtime.clone()
	}

	fn telemetry_config(&self) -> Option<scuffle_foundations::telemetry::settings::TelemetrySettings> {
		Some(self.telemetry.clone())
	}
}

mod config;
mod database;
mod disk;
mod event_queue;
mod global;
mod management;
mod worker;

#[bootstrap]
async fn main(cfg: Matches<ImageProcessorConfig>) {
	tracing::info!("starting image processor");

	// Require a health check to be registered
	scuffle_foundations::telemetry::server::require_health_check();

	let global = Arc::new({
		match global::Global::new(cfg.settings).await {
			Ok(global) => global,
			Err(err) => {
				tracing::error!("error setting up global: {err}");
				std::process::exit(1);
			}
		}
	});

	scuffle_foundations::telemetry::server::register_health_check(global.clone());

	let mut handles = Vec::new();

	if global.config().management.grpc.enabled || global.config().management.http.enabled {
		handles.push(runtime::spawn(management::start(global.clone())));
	}

	if global.config().worker.enabled {
		handles.push(runtime::spawn(worker::start(global.clone())));
	}

	let mut signal = scuffle_foundations::signal::SignalHandler::new()
		.with_signal(SignalKind::interrupt())
		.with_signal(SignalKind::terminate());

	let handles = futures::future::try_join_all(
		handles
			.iter_mut()
			.map(|handle| async move { handle.await.context("spawn task failed")? }),
	);

	tokio::select! {
		_ = signal.recv() => {
			tracing::info!("received signal, shutting down");
		}
		result = handles => {
			match result {
				Ok(_) => {
					tracing::warn!("handles completed unexpectedly without error");
				},
				Err(err) => tracing::error!("error in handle: {}", err),
			}
		}
	}

	let handle = scuffle_foundations::context::Handler::global();

	tokio::select! {
		_ = signal.recv() => {
			tracing::warn!("received signal again, forcing exit");
		},
		r = tokio::time::timeout(std::time::Duration::from_secs(60), handle.shutdown()) => {
			if r.is_err() {
				tracing::warn!("shutdown timed out, forcing exit");
			} else {
				tracing::info!("image processor stopped");
			}
		}
	}

	std::process::exit(0);
}
