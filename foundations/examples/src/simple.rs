use scuffle_foundations::bootstrap::{bootstrap, Bootstrap, RuntimeSettings};
use scuffle_foundations::settings::auto_settings;
use scuffle_foundations::settings::cli::Matches;
use scuffle_foundations::telemetry::metrics::metrics;
use scuffle_foundations::telemetry::settings::TelemetrySettings;

#[metrics]
mod http_server {
	use std::sync::Arc;

	use scuffle_foundations::telemetry::metrics::prometheus_client::metrics::counter::Counter;

	/// Number of active client connections.
	pub fn active_connections(endpoint_name: &str) -> Counter;

	/// Number of failed client connections.
	pub fn failed_connections_total(endpoint_name: &Arc<String>) -> Counter;

	/// Number of HTTP requests.
	/// xd
	pub fn requests_total(endpoint_name: &Arc<String>) -> Counter;

	/// Number of failed requests.
	pub fn requests_failed_total(endpoint_name: &Arc<String>, status_code: u16) -> Counter;
}

#[auto_settings]
pub struct HttpServerSettings {
	/// Telemetry Settings
	telemetry: TelemetrySettings,
	/// Runtime Settings
	runtime: RuntimeSettings,
}

impl Bootstrap for HttpServerSettings {
	type Settings = Self;

	fn runtime_mode(&self) -> RuntimeSettings {
		self.runtime.clone()
	}

	fn telemetry_config(&self) -> Option<TelemetrySettings> {
		Some(self.telemetry.clone())
	}
}

#[bootstrap]
async fn main(settings: Matches<HttpServerSettings>) {
	tracing::info!("hello world");

	dbg!(&settings);

	tokio::signal::ctrl_c().await.unwrap();
}
