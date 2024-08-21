use std::sync::Arc;

use ::opentelemetry::trace::TraceError;
use itertools::Itertools;
use opentelemetry_otlp::SpanExporter;
use thread_local::ThreadLocal;
use tokio::sync::{Mutex, OwnedSemaphorePermit};
#[cfg(not(feature = "runtime"))]
use tokio::task::spawn;

use super::layer::SpanHolder;
use super::node::SpanNode;
#[cfg(feature = "runtime")]
use crate::runtime::spawn;

#[cfg(feature = "metrics")]
#[crate::telemetry::metrics::metrics(crate_path = "crate")]
mod opentelemetry {
	use prometheus_client::metrics::counter::Counter;

	#[derive(serde::Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
	#[serde(rename_all = "snake_case")]
	pub enum SpanDroppedReason {
		ExportFailed,
		ExportTimeout,
		ThreadBackpressure,
		PendingExportBackpressure,
	}

	pub fn spans_exported() -> Counter;
	pub fn spans_dropped(reason: SpanDroppedReason) -> Counter;
}

pub struct BatchExporter {
	pub interval: tokio::time::Duration,
	pub batch_size: usize,
	pub max_concurrent_exports: usize,
	pub max_pending_exports: usize,
	#[cfg(feature = "metrics")]
	pub metrics: bool,
	pub error_handler: Box<dyn Fn(TraceError, usize) + Send + Sync>,
	pub drop_handler: Box<dyn Fn(usize) + Send + Sync>,
	pub export_handler: Box<dyn Fn(usize) + Send + Sync>,
}

impl BatchExporter {
	pub fn with_error_handler<F>(&mut self, handler: F) -> &mut Self
	where
		F: Fn(TraceError, usize) + Send + Sync + 'static,
	{
		self.error_handler = Box::new(handler);
		self
	}

	pub fn with_drop_handler<F>(&mut self, handler: F) -> &mut Self
	where
		F: Fn(usize) + Send + Sync + 'static,
	{
		self.drop_handler = Box::new(handler);
		self
	}

	pub fn with_export_handler<F>(&mut self, handler: F) -> &mut Self
	where
		F: Fn(usize) + Send + Sync + 'static,
	{
		self.export_handler = Box::new(handler);
		self
	}

	pub fn with_interval(&mut self, interval: tokio::time::Duration) -> &mut Self {
		self.interval = interval;
		self
	}

	pub fn with_batch_size(&mut self, batch_size: usize) -> &mut Self {
		self.batch_size = batch_size;
		self
	}

	pub fn with_max_concurrent_exports(&mut self, max_concurrent_exports: usize) -> &mut Self {
		self.max_concurrent_exports = max_concurrent_exports;
		self
	}

	pub fn with_max_pending_exports(&mut self, max_pending_exports: usize) -> &mut Self {
		self.max_pending_exports = max_pending_exports;
		self
	}

	pub fn build(&mut self) -> Self {
		std::mem::take(self)
	}
}

impl std::fmt::Debug for BatchExporter {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ExporterConfig")
			.field("interval", &self.interval)
			.field("batch_size", &self.batch_size)
			.field("max_concurrent_exports", &self.max_concurrent_exports)
			.field("max_pending_exports", &self.max_pending_exports)
			.finish()
	}
}

impl Default for BatchExporter {
	fn default() -> Self {
		Self {
			interval: tokio::time::Duration::from_secs(2),
			batch_size: 10_000,
			max_concurrent_exports: 10,
			max_pending_exports: 15,
			error_handler: Box::new(|err, count| {
				tracing::error!(err = %err, count, "failed to export spans");
			}),
			drop_handler: Box::new(|count| {
				tracing::warn!(count, "dropped spans");
			}),
			export_handler: Box::new(|count| {
				tracing::debug!(count, "exported spans");
			}),
			#[cfg(feature = "metrics")]
			metrics: true,
		}
	}
}

pub(super) struct Exporter {
	internal: Arc<ExportInternal>,
	span_buffer: Vec<Vec<SpanNode>>,
	spans: Arc<ThreadLocal<spin::Mutex<SpanHolder>>>,
}

struct ExportInternal {
	channel: Mutex<SpanExporter>,
	config: BatchExporter,
	concurrent_semaphore: tokio::sync::Semaphore,
	pending_semaphore: Arc<tokio::sync::Semaphore>,
}

fn export_batch(internal: Arc<ExportInternal>, batch: Vec<SpanNode>, pending_permit: OwnedSemaphorePermit) {
	use opentelemetry_sdk::export::trace::SpanExporter;

	spawn(async move {
		let _permit = internal.concurrent_semaphore.acquire().await.unwrap();
		drop(pending_permit);

		let batch = batch.into_iter().map(|data| data.into_data()).collect_vec();

		let size = batch.len();

		let fut = { internal.channel.lock().await.export(batch) };

		if let Err(err) = fut.await {
			#[cfg(feature = "metrics")]
			if internal.config.metrics {
				let reason = match err {
					TraceError::ExportTimedOut(_) => opentelemetry::SpanDroppedReason::ExportTimeout,
					_ => opentelemetry::SpanDroppedReason::ExportFailed,
				};

				opentelemetry::spans_dropped(reason).inc_by(size as u64);
			}

			(internal.config.error_handler)(err, size);
		} else {
			#[cfg(feature = "metrics")]
			if internal.config.metrics {
				opentelemetry::spans_exported().inc_by(size as u64);
			}

			(internal.config.export_handler)(size);
		}
	});
}

impl Exporter {
	pub fn new(channel: SpanExporter, config: BatchExporter, spans: Arc<ThreadLocal<spin::Mutex<SpanHolder>>>) -> Self {
		Self {
			internal: Arc::new(ExportInternal {
				channel: Mutex::new(channel),
				concurrent_semaphore: tokio::sync::Semaphore::new(config.max_concurrent_exports),
				pending_semaphore: Arc::new(tokio::sync::Semaphore::new(
					config.max_pending_exports.max(config.max_concurrent_exports),
				)),
				config,
			}),
			spans,
			span_buffer: Vec::new(),
		}
	}

	pub fn fetch_spans(&mut self) -> usize {
		let buffers = std::mem::take(&mut self.span_buffer)
			.into_iter()
			.chain(std::iter::repeat(Vec::new()));

		self.span_buffer.iter_mut().for_each(|spans| {
			spans.clear();
			spans.reserve_exact(self.internal.config.batch_size);
		});

		let mut total_dropped = 0;

		self.span_buffer = self
			.spans
			.iter()
			.zip(buffers)
			.map(|(spans, buffer)| {
				let mut spans = spans.lock();
				total_dropped += spans.drop_count();
				spans.reset_drop_count();

				spans.drain(buffer)
			})
			.collect();

		#[cfg(feature = "metrics")]
		if self.internal.config.metrics {
			opentelemetry::spans_dropped(opentelemetry::SpanDroppedReason::ThreadBackpressure).inc_by(total_dropped as u64);
		}

		total_dropped
	}

	pub async fn run(mut self) {
		tracing::debug!("starting exporter");

		loop {
			tokio::time::sleep(self.internal.config.interval).await;

			let thread_total_dropped = self.fetch_spans();

			let mut drop_pending = false;

			for chunk in self
				.span_buffer
				.iter_mut()
				.flat_map(|spans| spans.drain(..))
				.flat_map(|s| s.flatten())
				.chunks(self.internal.config.batch_size)
				.into_iter()
			{
				let Ok(pending_permit) = self.internal.pending_semaphore.clone().try_acquire_owned() else {
					drop_pending = true;
					break;
				};

				let chunk = chunk.collect_vec();
				tracing::debug!("exporting batch of {} spans", chunk.len());
				export_batch(self.internal.clone(), chunk, pending_permit);
			}

			let mut pending_total_dropped = 0;

			if drop_pending {
				self.span_buffer.iter_mut().for_each(|spans| {
					pending_total_dropped += spans.len();
					spans.clear();
				});
			}

			#[cfg(feature = "metrics")]
			if self.internal.config.metrics {
				opentelemetry::spans_dropped(opentelemetry::SpanDroppedReason::PendingExportBackpressure)
					.inc_by(pending_total_dropped as u64);
			}

			let total_dropped = thread_total_dropped + pending_total_dropped;

			if total_dropped > 0 {
				(self.internal.config.drop_handler)(total_dropped);
			}
		}
	}
}
