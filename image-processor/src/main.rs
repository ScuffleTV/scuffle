use std::sync::Arc;

use scuffle_foundations::bootstrap::{bootstrap, Bootstrap};
use scuffle_foundations::settings::cli::Matches;
use scuffle_foundations::BootstrapResult;

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

#[bootstrap]
async fn main(cfg: Matches<ImageProcessorConfig>) -> BootstrapResult<()> {
	tracing::info!("starting image processor");

	let global = Arc::new(global::Global::new(cfg.settings).await?);

	Ok(())
}
