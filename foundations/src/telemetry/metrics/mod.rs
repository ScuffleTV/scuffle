use std::collections::HashMap;

pub use prometheus_client;
#[cfg(all(feature = "macros", feature = "metrics"))]
pub use scuffle_foundations_macros::metrics;

#[doc(hidden)]
pub mod registries;

#[doc(hidden)]
pub mod serde;

pub fn init(service_info: crate::ServiceInfo, labels: &HashMap<String, String>) {
	registries::Registries::init(service_info, labels)
}

pub fn collect(collect_optional: bool) -> anyhow::Result<String> {
	let mut buffer = String::new();
	registries::Registries::collect(&mut buffer, collect_optional)?;
	Ok(buffer)
}

pub trait MetricBuilder<M: prometheus_client::metrics::TypedMetric> {
	fn build(&self) -> M;
}

#[derive(Debug, Clone, Copy)]
pub struct HistogramBuilder<const N: usize> {
	pub buckets: [f64; N],
}

impl Default for HistogramBuilder<11> {
	fn default() -> Self {
		Self {
			buckets: [0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0],
		}
	}
}

impl<const N: usize> MetricBuilder<prometheus_client::metrics::histogram::Histogram> for HistogramBuilder<N> {
	fn build(&self) -> prometheus_client::metrics::histogram::Histogram {
		prometheus_client::metrics::histogram::Histogram::new(self.buckets.iter().copied())
	}
}

impl<F, M> MetricBuilder<M> for F
where
	F: Fn() -> M,
	M: prometheus_client::metrics::TypedMetric,
{
	fn build(&self) -> M {
		self()
	}
}
