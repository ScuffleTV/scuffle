use std::net::SocketAddr;

use anyhow::Context;

#[derive(Debug, Clone)]
pub struct ServerSettings {
	pub builder: crate::http::server::ServerBuilder,
	#[cfg(feature = "pprof-cpu")]
	pub pprof_cpu_path: Option<String>,
	#[cfg(feature = "metrics")]
	pub metrics_path: Option<String>,
	#[cfg(feature = "health-check")]
	pub health_path: Option<String>,
	#[cfg(feature = "health-check")]
	pub health_timeout: Option<std::time::Duration>,
	#[cfg(feature = "context")]
	pub context: Option<crate::context::Context>,
}

impl Default for ServerSettings {
	fn default() -> Self {
		Self {
			builder: SocketAddr::from(([127, 0, 0, 1], 9000)).into(),
			#[cfg(feature = "pprof-cpu")]
			pprof_cpu_path: Some("/debug/pprof/profile".into()),
			#[cfg(feature = "metrics")]
			metrics_path: Some("/metrics".into()),
			#[cfg(feature = "health-check")]
			health_path: Some("/health".into()),
			#[cfg(feature = "health-check")]
			health_timeout: Some(std::time::Duration::from_secs(5)),
			#[cfg(feature = "context")]
			context: Some(crate::context::Context::global()),
		}
	}
}

#[derive(serde::Deserialize)]
#[serde(default)]
struct PprofCpuQuery {
	frequency: i32,
	blocklist: Vec<String>,
	seconds: u32,
}

impl Default for PprofCpuQuery {
	fn default() -> Self {
		Self {
			frequency: 100,
			blocklist: Vec::new(),
			seconds: 15,
		}
	}
}

#[cfg(feature = "pprof-cpu")]
async fn pprof_cpu(
	axum::extract::Query(query): axum::extract::Query<PprofCpuQuery>,
) -> axum::response::Response<axum::body::Body> {
	if query.frequency < 100 {
		return axum::response::Response::builder()
			.status(axum::http::StatusCode::BAD_REQUEST)
			.body("frequency must be greater than or equal to 100".into())
			.unwrap();
	}

	if query.seconds > 60 || query.seconds < 5 {
		return axum::response::Response::builder()
			.status(axum::http::StatusCode::BAD_REQUEST)
			.body("duration must be less than or equal to 60 seconds and greater than or equal to 5 seconds".into())
			.unwrap();
	}

	match tokio::task::spawn_blocking(move || {
		crate::telemetry::pprof::Cpu::new(query.frequency, &query.blocklist)
			.capture(std::time::Duration::from_secs(query.seconds as u64))
	})
	.await
	{
		Ok(Ok(contents)) => axum::response::Response::builder()
			.status(axum::http::StatusCode::OK)
			.header("content-type", "application/octet-stream")
			.header("content-disposition", "attachment; filename=\"profile.pb.gz\"")
			.body(contents.into())
			.unwrap(),
		Ok(Err(err)) => {
			tracing::error!(%err, "failed to capture pprof cpu profile");
			axum::response::Response::builder()
				.status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
				.body("failed to capture pprof cpu profile".into())
				.unwrap()
		}
		Err(err) => {
			tracing::error!(%err, "failed to spawn blocking task");
			axum::response::Response::builder()
				.status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
				.body("failed to spawn blocking task".into())
				.unwrap()
		}
	}
}

#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct MetricsQuery {
	optional: bool,
}

#[cfg(feature = "metrics")]
async fn metrics(
	axum::extract::Query(query): axum::extract::Query<MetricsQuery>,
) -> axum::response::Response<axum::body::Body> {
	match tokio::task::spawn_blocking(move || crate::telemetry::metrics::collect(query.optional)).await {
		Ok(Ok(contents)) => axum::response::Response::builder()
			.status(axum::http::StatusCode::OK)
			.header("content-type", "text/plain")
			.body(contents.into())
			.unwrap(),
		Ok(Err(err)) => {
			tracing::error!(%err, "failed to collect metrics");
			axum::response::Response::builder()
				.status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
				.body("failed to collect metrics".into())
				.unwrap()
		}
		Err(err) => {
			tracing::error!(%err, "failed to spawn blocking task");
			axum::response::Response::builder()
				.status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
				.body("failed to spawn blocking task".into())
				.unwrap()
		}
	}
}

#[cfg(feature = "health-check")]
pub use health_check::{
	register as register_health_check, require as require_health_check, unregister as unregister_health_check, HealthCheck,
	HealthCheckFn,
};

#[cfg(feature = "health-check")]
mod health_check {
	use std::pin::Pin;
	use std::sync::atomic::{AtomicBool, AtomicUsize};

	use futures::Future;
	use scc::HashMap;

	pub struct HealthCheckFn<F>(pub F);

	impl<F, Fut> HealthCheck for HealthCheckFn<F>
	where
		F: Fn() -> Fut + Send + Sync + 'static,
		Fut: Future<Output = bool> + Send + 'static,
	{
		fn check(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
			Box::pin((self.0)())
		}
	}

	pub trait HealthCheck: Send + Sync + 'static {
		fn check(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>>;
	}

	impl<H: HealthCheck> HealthCheck for std::sync::Arc<H> {
		fn check(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
			self.as_ref().check()
		}
	}

	impl<H: HealthCheck> HealthCheck for Box<H> {
		fn check(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
			self.as_ref().check()
		}
	}

	#[derive(Default)]
	struct HealthChecker {
		id: AtomicUsize,
		require_check: AtomicBool,
		health_checks: HashMap<usize, Box<dyn HealthCheck>>,
	}

	static HEALTH_CHECK: once_cell::sync::Lazy<HealthChecker> =
		once_cell::sync::Lazy::<HealthChecker>::new(HealthChecker::default);

	/// Register a health check and return an id
	pub fn register(check: impl HealthCheck) -> usize {
		let id = HEALTH_CHECK.id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
		HEALTH_CHECK
			.health_checks
			.insert(id, Box::new(check))
			.ok()
			.expect("id already exists");
		id
	}

	/// Unregister a health check by id
	pub fn unregister(id: usize) {
		HEALTH_CHECK.health_checks.remove(&id);
	}

	/// Require a health check to be registered, if no health checks are
	/// registered the server will always return 503 Service Unavailable This is
	/// useful for ensuring that the server is healthy before accepting traffic
	pub fn require() {
		HEALTH_CHECK.require_check.store(true, std::sync::atomic::Ordering::Relaxed);
	}

	pub async fn is_healthy() -> bool {
		if HEALTH_CHECK.require_check.load(std::sync::atomic::Ordering::Relaxed) && HEALTH_CHECK.health_checks.is_empty() {
			return false;
		}

		let mut o_entry = HEALTH_CHECK.health_checks.first_entry_async().await;

		while let Some(entry) = o_entry {
			if (entry.get()).check().await {
				return false;
			}

			o_entry = entry.next_async().await;
		}

		true
	}
}

#[cfg(feature = "health-check")]
async fn health(
	axum::Extension(timeout): axum::Extension<Option<std::time::Duration>>,
) -> axum::response::Response<axum::body::Body> {
	let healthy = if let Some(timeout) = timeout {
		tokio::time::timeout(timeout, health_check::is_healthy())
			.await
			.map_err(|err| {
				tracing::error!(%err, "failed to check health, timed out");
			})
			.unwrap_or(false)
	} else {
		health_check::is_healthy().await
	};

	if healthy {
		axum::response::Response::builder()
			.status(axum::http::StatusCode::OK)
			.body("ok".into())
			.unwrap()
	} else {
		axum::response::Response::builder()
			.status(axum::http::StatusCode::SERVICE_UNAVAILABLE)
			.body("unavailable".into())
			.unwrap()
	}
}

async fn not_found() -> &'static str {
	"not found"
}

#[tracing::instrument(name = "telemetry::server", skip(settings))]
pub async fn init(settings: ServerSettings) -> anyhow::Result<()> {
	let mut router = axum::routing::Router::new();

	#[cfg(feature = "pprof-cpu")]
	if let Some(path) = &settings.pprof_cpu_path {
		router = router.route(path, axum::routing::get(pprof_cpu));
	}

	#[cfg(feature = "metrics")]
	if let Some(path) = &settings.metrics_path {
		router = router.route(path, axum::routing::get(metrics));
	}

	#[cfg(feature = "health-check")]
	if let Some(path) = &settings.health_path {
		router = router
			.layer(axum::Extension(settings.health_timeout))
			.route(path, axum::routing::get(health));
	}

	router = router.fallback(axum::routing::any(not_found));

	let mut server = settings.builder.build(router).context("failed to build server")?;

	server.start_and_wait().await.context("failed to start server")?;

	Ok(())
}
