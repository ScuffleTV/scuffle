use std::net::SocketAddr;

use common::config::TlsConfig;

#[derive(Debug, Clone, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
pub struct ApiConfig {
	/// Bind Address
	pub bind_address: SocketAddr,

	/// The event stream to use
	pub event_stream: String,

	/// If we should use TLS
	pub tls: Option<TlsConfig>,
}

impl Default for ApiConfig {
	fn default() -> Self {
		Self {
			bind_address: "[::]:9080".to_string().parse().unwrap(),
			event_stream: "scuffle_video_events".to_string(),
			tls: None,
		}
	}
}
