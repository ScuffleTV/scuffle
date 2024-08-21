#[cfg(any(feature = "opentelemetry", feature = "logging"))]
use std::collections::HashMap;
#[cfg(feature = "telemetry-server")]
use std::net::SocketAddr;

#[cfg(feature = "opentelemetry")]
use opentelemetry_otlp::WithExportConfig;
#[cfg(feature = "opentelemetry")]
use opentelemetry_sdk::Resource;
#[cfg(feature = "logging")]
use tracing_subscriber::fmt::time::{ChronoLocal, ChronoUtc};

#[cfg(feature = "logging")]
use crate::telemetry::logging::TimeFormatter;
#[cfg(feature = "opentelemetry")]
use crate::telemetry::opentelemetry::{complex_rate_sampler, BatchExporter, Sampler, SpanObserver};

#[crate::settings::auto_settings(crate_path = "crate")]
pub struct TelemetrySettings {
	/// Settings for metric exporting.
	#[cfg(feature = "metrics")]
	pub metrics: MetricsSettings,
	/// Settings for opentelemetry span exporting.
	#[cfg(feature = "opentelemetry")]
	pub opentelemetry: OpentelemetrySettings,
	/// Settings for logging.
	#[cfg(feature = "logging")]
	pub logging: LoggingSettings,
	/// Settings for the http server.
	#[cfg(feature = "telemetry-server")]
	pub server: ServerSettings,
}

#[cfg(feature = "metrics")]
#[crate::settings::auto_settings(crate_path = "crate")]
pub struct MetricsSettings {
	/// Whether to enable metrics.
	#[settings(default = true)]
	pub enabled: bool,
	/// A map of additional labels to add to metrics.
	pub labels: HashMap<String, String>,
}

#[cfg(feature = "opentelemetry")]
#[crate::settings::auto_settings(crate_path = "crate")]
#[serde(default)]
pub struct OpentelemetrySettings {
	/// Whether to enable opentelemetry span exporting.
	#[settings(default = false)]
	pub enabled: bool,
	/// A map of additional labels to add to opentelemetry spans.
	pub labels: HashMap<String, String>,
	/// The max number of spans that have not started exporting.
	/// This value is a per-thread limit.
	#[settings(default = 500)]
	pub max_backpressure: usize,
	/// The number of spans to export in a batch.
	#[settings(default = 10_000)]
	pub batch_size: usize,
	/// The max number of concurrent batch exports.
	#[settings(default = 10)]
	pub max_concurrent_exports: usize,
	/// The max number of pending batch exports.
	#[settings(default = 15)]
	pub max_pending_exports: usize,
	/// The interval to export spans at.
	#[settings(default = std::time::Duration::from_secs(2))]
	#[serde(with = "humantime_serde")]
	pub interval: std::time::Duration,
	/// Sampler to use for picking which spans to export.
	#[settings(default = OpentelemetrySettingsSampler::Always)]
	pub sampler: OpentelemetrySettingsSampler,
	/// Export timeout.
	#[settings(default = std::time::Duration::from_secs(15))]
	#[serde(with = "humantime_serde")]
	pub otlp_timeout: std::time::Duration,
	/// The endpoint to export spans to.
	#[settings(default = "http://localhost:4317".into())]
	pub otlp_endpoint: String,
	/// The export method to use.
	#[settings(default = OpentelemetrySettingsExportMethod::Grpc)]
	pub otlp_method: OpentelemetrySettingsExportMethod,
	/// Filter to use for filtering spans.
	#[cfg_attr(
		feature = "env-filter",
		doc = "See https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html"
	)]
	#[cfg_attr(
		not(feature = "env-filter"),
		doc = "See https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.LevelFilter.html"
	)]
	#[settings(default = "info".into())]
	pub level: String,
	/// Export Logging Level
	pub logging: OpentelemetrySettingsLogging,
	/// Enable metrics for opentelemetry.
	#[cfg(feature = "metrics")]
	#[settings(default = true)]
	pub metrics: bool,
}

#[cfg(feature = "opentelemetry")]
#[crate::settings::auto_settings(crate_path = "crate")]
#[serde(default)]
pub struct OpentelemetrySettingsLogging {
	#[settings(default = OpentelemetrySettingsLoggingLevel::Warn)]
	pub dropped_spans: OpentelemetrySettingsLoggingLevel,
	#[settings(default = OpentelemetrySettingsLoggingLevel::Error)]
	pub exporter_errors: OpentelemetrySettingsLoggingLevel,
	#[settings(default = OpentelemetrySettingsLoggingLevel::Debug)]
	pub exporter_success: OpentelemetrySettingsLoggingLevel,
}

#[cfg(feature = "opentelemetry")]
#[crate::settings::auto_settings(crate_path = "crate")]
#[serde(rename_all = "lowercase")]
pub enum OpentelemetrySettingsLoggingLevel {
	/// Error level logging.
	Error,
	/// Warning level logging.
	Warn,
	#[settings(default)]
	/// Info level logging.
	Info,
	/// Debug level logging.
	Debug,
	/// Trace level logging.
	Trace,
	/// No logging.
	Off,
}

#[cfg(feature = "opentelemetry")]
#[crate::settings::auto_settings(crate_path = "crate")]
#[serde(rename_all = "lowercase")]
pub enum OpentelemetrySettingsExportMethod {
	#[settings(default)]
	/// Export spans over gRPC.
	Grpc,
	/// Export spans over HTTP.
	Http,
}

#[cfg(feature = "opentelemetry")]
#[crate::settings::auto_settings(crate_path = "crate")]
#[serde(rename_all = "kebab-case")]
pub enum OpentelemetrySettingsSampler {
	/// Always sample all spans.
	#[settings(default)]
	Always,
	/// Never sample any spans.
	Never,
	/// Sample spans based on a rate.
	RatioSimple(f64),
	/// Sample spans based on a rate, with the ability to set a different rate
	/// for root spans. This is useful because you can always sample root spans
	/// and then on some rate cull the tail. In production, you might want to
	/// sample all root spans and then sample tail spans at a lower rate.
	RatioComplex {
		/// The rate to sample spans at.
		head_rate: f64,
		/// The rate to sample root spans at.
		/// Root spans are spans that are not children of any other span.
		/// If `None`, the root rate is the same as the rate.
		tail_rate: Option<f64>,
		/// Error rate to sample spans at.
		/// If `None`, the error rate is the same as the rate.
		error_rate: Option<f64>,
		/// Sample all if any span in the tree contains an error.
		#[serde(default = "default_true")]
		sample_on_error: bool,
	},
}

fn default_true() -> bool {
	true
}

#[cfg(feature = "logging")]
#[crate::settings::auto_settings(crate_path = "crate")]
#[serde(default)]
pub struct LoggingSettings {
	/// Whether to enable logging.
	#[settings(default = true)]
	pub enabled: bool,
	/// The log level to filter logs by.
	#[cfg_attr(
		feature = "env-filter",
		doc = "See https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html"
	)]
	#[cfg_attr(
		not(feature = "env-filter"),
		doc = "See https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.LevelFilter.html"
	)]
	#[settings(default = "info".into())]
	pub level: String,
	/// The log format to use.
	#[settings(default = LoggingSettingsFormat::Normal)]
	pub format: LoggingSettingsFormat,
	/// Show spans in logs.
	#[settings(default = true)]
	pub show_spans: bool,
	/// Show the thread id in logs.
	#[settings(default = true)]
	pub show_thread_id: bool,
	/// Show the file info in logs.
	#[settings(default = true)]
	pub show_file_info: bool,
	/// Show timestamps in logs.
	#[settings(default = LoggingSettingsTimestamps::Local)]
	pub timestamps: LoggingSettingsTimestamps,
}

#[cfg(feature = "logging")]
#[crate::settings::auto_settings(crate_path = "crate")]
#[serde(rename_all = "lowercase")]
pub enum LoggingSettingsTimestamps {
	/// Show timestamps in logs in the local timezone.
	#[settings(default)]
	Local,
	/// Show timestamps in logs in UTC.
	Utc,
	/// Do not show timestamps in logs.
	Off,
}

#[cfg(feature = "logging")]
#[crate::settings::auto_settings(crate_path = "crate")]
#[serde(rename_all = "lowercase")]
pub enum LoggingSettingsFormat {
	#[settings(default)]
	/// The default human-readable log format.
	Normal,
	/// The JSON log format.
	Json,
	/// The pretty log format.
	Pretty,
	/// The compact log format.
	Compact,
}

#[cfg(all(any(feature = "pprof-cpu", feature = "metrics"), feature = "telemetry-server"))]
#[crate::settings::auto_settings(crate_path = "crate")]
#[serde(default)]
pub struct ServerSettings {
	/// Whether to enable the server.
	#[settings(default = true)]
	pub enabled: bool,
	/// The address to bind the server to.
	#[settings(default = SocketAddr::from(([127, 0, 0, 1], 9000)))]
	pub bind: SocketAddr,
	/// The path to the pprof CPU endpoint. If `None`, the endpoint is disabled.
	#[cfg(feature = "pprof-cpu")]
	#[settings(default = Some("/debug/pprof/profile".into()))]
	pub pprof_cpu_path: Option<String>,
	/// The path to the metrics endpoint. If `None`, the endpoint is disabled.
	#[cfg(feature = "metrics")]
	#[settings(default = Some("/metrics".into()))]
	pub metrics_path: Option<String>,
	/// The path to use for the health check endpoint. If `None`, the endpoint
	/// is disabled.
	#[cfg(feature = "health-check")]
	#[settings(default = Some("/health".into()))]
	pub health_path: Option<String>,
	/// Health check timeout.
	#[cfg(feature = "health-check")]
	#[settings(default = Some(std::time::Duration::from_secs(5)))]
	#[serde(with = "humantime_serde")]
	pub health_timeout: Option<std::time::Duration>,
}

pub async fn init(info: crate::ServiceInfo, settings: TelemetrySettings) {
	#[cfg(feature = "metrics")]
	if settings.metrics.enabled {
		crate::telemetry::metrics::init(info, &settings.metrics.labels);
	}

	#[cfg(any(feature = "opentelemetry", feature = "logging"))]
	{
		#[cfg(feature = "opentelemetry")]
		let opentelemetry = if settings.opentelemetry.enabled {
			Some(
				crate::telemetry::opentelemetry::layer(
					SpanObserver {
						max_unprocessed_spans_per_thread: settings.opentelemetry.max_backpressure,
						sampler: match settings.opentelemetry.sampler {
							OpentelemetrySettingsSampler::Always => Sampler::Always,
							OpentelemetrySettingsSampler::Never => Sampler::Never,
							OpentelemetrySettingsSampler::RatioSimple(rate) => Sampler::TraceIdRatio(rate),
							OpentelemetrySettingsSampler::RatioComplex {
								tail_rate,
								head_rate,
								error_rate,
								sample_on_error,
							} => complex_rate_sampler(head_rate, tail_rate, error_rate, sample_on_error),
						},
					},
					BatchExporter {
						batch_size: settings.opentelemetry.batch_size,
						max_concurrent_exports: settings.opentelemetry.max_concurrent_exports,
						max_pending_exports: settings.opentelemetry.max_pending_exports,
						interval: settings.opentelemetry.interval,
						#[cfg(feature = "metrics")]
						metrics: settings.opentelemetry.metrics,
						drop_handler: {
							const DROPPED_SPANS_ERROR: &str = "opentelemetry exporter dropped spans due to backpressure";

							match settings.opentelemetry.logging.dropped_spans {
								OpentelemetrySettingsLoggingLevel::Error => Box::new(|count| {
									tracing::error!(count, DROPPED_SPANS_ERROR);
								}),
								OpentelemetrySettingsLoggingLevel::Warn => Box::new(|count| {
									tracing::warn!(count, DROPPED_SPANS_ERROR);
								}),
								OpentelemetrySettingsLoggingLevel::Info => Box::new(|count| {
									tracing::info!(count, DROPPED_SPANS_ERROR);
								}),
								OpentelemetrySettingsLoggingLevel::Debug => Box::new(|count| {
									tracing::debug!(count, DROPPED_SPANS_ERROR);
								}),
								OpentelemetrySettingsLoggingLevel::Trace => Box::new(|count| {
									tracing::trace!(count, DROPPED_SPANS_ERROR);
								}),
								OpentelemetrySettingsLoggingLevel::Off => Box::new(|_| {}),
							}
						},
						error_handler: {
							const EXPORTER_ERROR: &str = "opentelemetry exporter failed to export spans";

							match settings.opentelemetry.logging.exporter_errors {
								OpentelemetrySettingsLoggingLevel::Error => Box::new(|err, count| {
									tracing::error!(err = %err, count, EXPORTER_ERROR);
								}),
								OpentelemetrySettingsLoggingLevel::Warn => Box::new(|err, count| {
									tracing::warn!(err = %err, count, EXPORTER_ERROR);
								}),
								OpentelemetrySettingsLoggingLevel::Info => Box::new(|err, count| {
									tracing::info!(err = %err, count, EXPORTER_ERROR);
								}),
								OpentelemetrySettingsLoggingLevel::Debug => Box::new(|err, count| {
									tracing::debug!(err = %err, count, EXPORTER_ERROR);
								}),
								OpentelemetrySettingsLoggingLevel::Trace => Box::new(|err, count| {
									tracing::trace!(err = %err, count, EXPORTER_ERROR);
								}),
								OpentelemetrySettingsLoggingLevel::Off => Box::new(|_, _| {}),
							}
						},
						export_handler: {
							const EXPORTER_SUCCESS: &str = "opentelemetry exporter successfully exported spans";

							match settings.opentelemetry.logging.exporter_success {
								OpentelemetrySettingsLoggingLevel::Error => Box::new(|count| {
									tracing::error!(count, EXPORTER_SUCCESS);
								}),
								OpentelemetrySettingsLoggingLevel::Warn => Box::new(|count| {
									tracing::warn!(count, EXPORTER_SUCCESS);
								}),
								OpentelemetrySettingsLoggingLevel::Info => Box::new(|count| {
									tracing::info!(count, EXPORTER_SUCCESS);
								}),
								OpentelemetrySettingsLoggingLevel::Debug => Box::new(|count| {
									tracing::debug!(count, EXPORTER_SUCCESS);
								}),
								OpentelemetrySettingsLoggingLevel::Trace => Box::new(|count| {
									tracing::trace!(count, EXPORTER_SUCCESS);
								}),
								OpentelemetrySettingsLoggingLevel::Off => Box::new(|_| {}),
							}
						},
					},
					{
						let mut exporter = match settings.opentelemetry.otlp_method {
							OpentelemetrySettingsExportMethod::Grpc => opentelemetry_otlp::new_exporter()
								.tonic()
								.with_endpoint(settings.opentelemetry.otlp_endpoint.clone())
								.with_timeout(settings.opentelemetry.otlp_timeout)
								.build_span_exporter(),
							OpentelemetrySettingsExportMethod::Http => opentelemetry_otlp::new_exporter()
								.http()
								.with_endpoint(settings.opentelemetry.otlp_endpoint.clone())
								.with_timeout(settings.opentelemetry.otlp_timeout)
								.build_span_exporter(),
						}
						.expect("failed to build otlp exporter");

						use opentelemetry_sdk::export::trace::SpanExporter;

						exporter.set_resource(&{
							let mut kv = vec![];

							if !settings.opentelemetry.labels.contains_key("service.name") {
								kv.push(opentelemetry::KeyValue::new("service.name", info.metric_name));
							}

							if !settings.opentelemetry.labels.contains_key("service.version") {
								kv.push(opentelemetry::KeyValue::new("service.version", info.version));
							}

							kv.extend(
								settings
									.opentelemetry
									.labels
									.iter()
									.map(|(k, v)| opentelemetry::KeyValue::new(k.clone(), v.clone())),
							);

							Resource::new(kv)
						});

						exporter
					},
				)
				.with_filter(super::LevelFilter::new(&settings.opentelemetry.level).filter()),
			)
		} else {
			None
		};

		#[cfg(feature = "logging")]
		let logging = if settings.logging.enabled {
			let layer = tracing_subscriber::fmt::layer()
				.with_file(settings.logging.show_file_info)
				.with_line_number(settings.logging.show_file_info)
				.with_thread_ids(settings.logging.show_thread_id)
				.with_timer(match settings.logging.timestamps {
					LoggingSettingsTimestamps::Local => TimeFormatter::Local(ChronoLocal::rfc_3339()),
					LoggingSettingsTimestamps::Utc => TimeFormatter::Utc(ChronoUtc::rfc_3339()),
					LoggingSettingsTimestamps::Off => TimeFormatter::None,
				});

			let layer = match settings.logging.format {
				LoggingSettingsFormat::Normal => layer.boxed(),
				LoggingSettingsFormat::Json => layer.json().boxed(),
				LoggingSettingsFormat::Pretty => layer.pretty().boxed(),
				LoggingSettingsFormat::Compact => layer.compact().boxed(),
			};

			Some(layer.with_filter(super::LevelFilter::new(&settings.logging.level).filter()))
		} else {
			None
		};

		use tracing_subscriber::prelude::*;

		let registry = tracing_subscriber::registry();
		#[cfg(feature = "opentelemetry")]
		let registry = registry.with(opentelemetry);
		#[cfg(feature = "logging")]
		let registry = registry.with(logging);
		registry.init();
	}

	#[cfg(all(any(feature = "pprof-cpu", feature = "metrics"), feature = "telemetry-server"))]
	if settings.server.enabled {
		#[cfg(not(feature = "runtime"))]
		use tokio::spawn;

		#[cfg(feature = "runtime")]
		use crate::runtime::spawn;

		spawn(async move {
			match crate::telemetry::server::init(super::server::ServerSettings {
				builder: settings.server.bind.into(),
				#[cfg(feature = "metrics")]
				metrics_path: settings.server.metrics_path,
				#[cfg(feature = "pprof-cpu")]
				pprof_cpu_path: settings.server.pprof_cpu_path,
				#[cfg(feature = "health-check")]
				health_path: settings.server.health_path,
				#[cfg(feature = "health-check")]
				health_timeout: settings.server.health_timeout,
				#[cfg(feature = "context")]
				context: Some(crate::context::Context::global()),
			})
			.await
			{
				Ok(()) => {}
				Err(err) => {
					tracing::error!(error = %err, "failed to start server");
				}
			}
		});
	}
}
