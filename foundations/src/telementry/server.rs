use std::net::SocketAddr;

use anyhow::Context;

#[derive(Debug)]
pub struct ServerSettings {
	pub bind: SocketAddr,
	#[cfg(feature = "pprof-cpu")]
	pub pprof_cpu_path: Option<String>,
	#[cfg(feature = "pprof-heap")]
	pub pprof_heap_path: Option<String>,
	#[cfg(feature = "metrics")]
	pub metrics_path: Option<String>,
	#[cfg(feature = "health-check")]
	pub health_path: Option<String>,
	#[cfg(feature = "context")]
	pub context: Option<crate::context::Context>,
}

impl Default for ServerSettings {
	fn default() -> Self {
		Self {
			bind: SocketAddr::from(([127, 0, 0, 1], 9090)),
			#[cfg(feature = "pprof-cpu")]
			pprof_cpu_path: Some("/debug/pprof/profile".into()),
			#[cfg(feature = "pprof-heap")]
			pprof_heap_path: Some("/debug/pprof/heap".into()),
			#[cfg(feature = "metrics")]
			metrics_path: Some("/metrics".into()),
			#[cfg(feature = "health-check")]
			health_path: Some("/health".into()),
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
		crate::telementry::pprof::Cpu::new(query.frequency, &query.blocklist)
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

#[cfg(feature = "pprof-heap")]
async fn pprof_heap() -> axum::response::Response<axum::body::Body> {
	match tokio::task::spawn_blocking(|| crate::telementry::pprof::Heap::new().capture()).await {
		Ok(Ok(contents)) => axum::response::Response::builder()
			.status(axum::http::StatusCode::OK)
			.header("content-type", "application/octet-stream")
			.header("content-disposition", "attachment; filename=\"heap.pb.gz\"")
			.body(contents.into())
			.unwrap(),
		Ok(Err(err)) => {
			tracing::error!(%err, "failed to capture pprof heap profile");
			axum::response::Response::builder()
				.status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
				.body("failed to capture pprof heap profile".into())
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
	match tokio::task::spawn_blocking(move || crate::telementry::metrics::collect(query.optional)).await {
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
pub use health_check::{register as register_health_check, unregister as unregister_health_check, HealthCheck};

#[cfg(feature = "health-check")]
mod health_check {
	use std::pin::Pin;
	use std::sync::atomic::AtomicUsize;

	use futures::Future;
	use scc::HashMap;

	pub trait HealthCheck: Send + Sync + 'static {
		fn check(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>>;
	}

	impl<F, Fut> HealthCheck for F
	where
		F: Fn() -> Fut + Send + Sync + 'static,
		Fut: Future<Output = bool> + Send + Sync + 'static,
	{
		fn check(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
			Box::pin((self)())
		}
	}

	#[derive(Default)]
	struct HealthChecker {
		id: AtomicUsize,
		health_checks: HashMap<usize, Box<dyn HealthCheck>>,
	}

	static HEALTH_CHECK: once_cell::sync::Lazy<HealthChecker> =
		once_cell::sync::Lazy::<HealthChecker>::new(|| HealthChecker::default());

	pub fn register(check: impl HealthCheck) -> usize {
		let id = HEALTH_CHECK.id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
		HEALTH_CHECK
			.health_checks
			.insert(id, Box::new(check))
			.ok()
			.expect("id already exists");
		id
	}

	pub fn unregister(id: usize) {
		HEALTH_CHECK.health_checks.remove(&id);
	}

	pub async fn is_healthy() -> bool {
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
async fn health() -> axum::response::Response<axum::body::Body> {
	if health_check::is_healthy().await {
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

pub async fn init(settings: ServerSettings) -> anyhow::Result<()> {
	let mut router = axum::routing::Router::new();

	#[cfg(feature = "pprof-cpu")]
	if let Some(path) = &settings.pprof_cpu_path {
		router = router.route(path, axum::routing::get(pprof_cpu));
	}

	#[cfg(feature = "pprof-heap")]
	if let Some(path) = &settings.pprof_heap_path {
		router = router.route(path, axum::routing::get(pprof_heap));
	}

	#[cfg(feature = "metrics")]
	if let Some(path) = &settings.metrics_path {
		router = router.route(path, axum::routing::get(metrics));
	}

	#[cfg(feature = "health-check")]
	if let Some(path) = &settings.health_path {
		router = router.route(path, axum::routing::get(health));
	}

	router = router.fallback(axum::routing::any(not_found));

	let tcp_listener = tokio::net::TcpListener::bind(settings.bind)
		.await
		.context("failed to bind tcp listener")?;

	tracing::info!("telemetry server listening on {}", settings.bind);

	let server = axum::serve(tcp_listener, router);

	#[cfg(feature = "context")]
	let server = server.with_graceful_shutdown(async move {
		if let Some(context) = settings.context {
			context.done().await;
		} else {
			std::future::pending::<()>().await;
		}
	});

	server.await.context("failed to serve")
}
