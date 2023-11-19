use std::collections::HashMap;
use std::net::SocketAddr;

use common::config::TlsConfig;

use crate::ratelimit::RateLimitResource;

#[derive(Debug, Clone, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
pub struct ApiConfig {
	/// Bind Address
	pub bind_address: SocketAddr,

	/// The event stream to use
	pub event_stream: String,

	/// The stream to use for recording delete events
	pub recording_delete_stream: String,

	/// The batch size for deleting recordings
	pub recording_delete_batch_size: usize,

	/// If we should use TLS
	pub tls: Option<TlsConfig>,

	/// The ratelimit rules
	pub rate_limit_rules: RatelimitRules,
}

#[derive(Debug, Default, Clone, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
pub struct RatelimitRules {
	/// The default rules
	pub default: RatelimitRule,

	/// Banned and exceeded rules
	pub banned_exceeded: RatelimitBannedExceededRules,

	/// The custom rules
	#[config(cli(skip), env(skip))]
	pub rules: HashMap<RateLimitResource, RatelimitRule>,
}

#[derive(Debug, Clone, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
pub struct RatelimitBannedExceededRules {
	/// The number of exceeded requests before the user is banned
	pub exceeded_limit: u32,
	/// The amount of time before exceeded is reset in seconds
	pub exceeded_reset_seconds: u32,
	/// The amount of time before the user is unbanned in seconds
	pub banned_reset_seconds: u32,
}

impl Default for RatelimitBannedExceededRules {
	fn default() -> Self {
		Self {
			exceeded_limit: 1000,
			exceeded_reset_seconds: 180,
			banned_reset_seconds: 900,
		}
	}
}

#[derive(Debug, Clone, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
pub struct RatelimitRule {
	/// The cost of the request
	pub cost: u32,
	/// The allowed quota for the duration
	pub quota: u32,
	/// How often the quota is reset in seconds
	pub quota_reset_seconds: u32,
	/// Failed restore cost
	pub failed_cost: u32,
}

impl Default for RatelimitRule {
	fn default() -> Self {
		Self {
			cost: 10,
			quota: 1000,
			quota_reset_seconds: 30,
			failed_cost: 1,
		}
	}
}

impl Default for ApiConfig {
	fn default() -> Self {
		Self {
			bind_address: "[::]:9080".to_string().parse().unwrap(),
			event_stream: "scuffle_video_events".to_string(),
			tls: None,
			recording_delete_stream: "scuffle_video_recording_delete".to_string(),
			recording_delete_batch_size: 1000,
			rate_limit_rules: RatelimitRules::default(),
		}
	}
}
