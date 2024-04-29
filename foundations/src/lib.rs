#[cfg(feature = "runtime")]
pub mod runtime;

#[cfg(feature = "heap")]
pub mod heap;

#[cfg(feature = "macros")]
pub use scuffle_foundations_macros::wrapped;

#[cfg(feature = "macros")]
#[doc(hidden)]
pub mod macro_reexports {
	#[cfg(feature = "cli")]
	pub use const_str;
	#[cfg(feature = "metrics")]
	pub use once_cell;
	#[cfg(feature = "metrics")]
	pub use parking_lot;
	#[cfg(feature = "metrics")]
	pub use prometheus_client;
	#[cfg(any(feature = "settings", feature = "metrics"))]
	pub use serde;
}

pub type BootstrapResult<T> = anyhow::Result<T>;

#[cfg(feature = "settings")]
pub mod settings;

#[cfg(feature = "bootstrap")]
pub mod bootstrap;

#[cfg(feature = "_telemetry")]
pub mod telementry;

#[cfg(feature = "signal")]
pub mod signal;

#[cfg(feature = "context")]
pub mod context;

#[derive(Debug, Clone, Copy, Default)]
/// Information about the service.
pub struct ServiceInfo {
	/// The name of the service.
	pub name: &'static str,
	/// The name of the service for metrics. Replaces `-` with `_`.
	pub metric_name: &'static str,
	/// The version of the service.
	pub version: &'static str,
	/// The author of the service.
	pub author: &'static str,
	/// A description of the service.
	pub description: &'static str,
}

#[macro_export]
macro_rules! service_info {
	() => {
		$crate::ServiceInfo {
			name: env!("CARGO_PKG_NAME"),
			metric_name: $crate::macro_reexports::const_str::replace!(env!("CARGO_PKG_NAME"), "-", "_"),
			version: env!("CARGO_PKG_VERSION"),
			author: env!("CARGO_PKG_AUTHORS"),
			description: env!("CARGO_PKG_DESCRIPTION"),
		}
	};
}
