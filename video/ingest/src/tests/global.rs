use std::collections::HashMap;
use std::sync::Arc;

use common::context::{Context, Handler};
use common::logging;
use tokio::sync::{mpsc, Mutex};
use ulid::Ulid;

use crate::config::IngestConfig;
use crate::global::IncomingTranscoder;

pub struct GlobalState {
	ctx: Context,
	config: IngestConfig,
	nats: async_nats::Client,
	jetstream: async_nats::jetstream::Context,
	db: Arc<sqlx::PgPool>,
	requests: Mutex<HashMap<Ulid, mpsc::Sender<IncomingTranscoder>>>,
}

impl common::global::GlobalCtx for GlobalState {
	fn ctx(&self) -> &Context {
		&self.ctx
	}
}

impl common::global::GlobalConfigProvider<IngestConfig> for GlobalState {
	fn provide_config(&self) -> &IngestConfig {
		&self.config
	}
}

impl common::global::GlobalNats for GlobalState {
	fn nats(&self) -> &async_nats::Client {
		&self.nats
	}

	fn jetstream(&self) -> &async_nats::jetstream::Context {
		&self.jetstream
	}
}

impl common::global::GlobalDb for GlobalState {
	fn db(&self) -> &Arc<sqlx::PgPool> {
		&self.db
	}
}

impl common::global::GlobalConfig for GlobalState {}

impl crate::global::IngestState for GlobalState {
	fn requests(&self) -> &Mutex<HashMap<Ulid, mpsc::Sender<IncomingTranscoder>>> {
		&self.requests
	}
}

pub async fn mock_global_state(config: IngestConfig) -> (Arc<GlobalState>, Handler) {
	let (ctx, handler) = Context::new();

	dotenvy::dotenv().ok();

	let logging_level = std::env::var("LOGGING_LEVEL").unwrap_or_else(|_| "info".to_string());

	logging::init(&logging_level, Default::default()).expect("failed to initialize logging");

	let database_uri = std::env::var("VIDEO_DATABASE_URL").expect("DATABASE_URL must be set");
	let nats_addr = std::env::var("NATS_ADDR").expect("NATS_URL must be set");

	let nats = async_nats::connect(&nats_addr).await.expect("failed to connect to nats");
	let jetstream = async_nats::jetstream::new(nats.clone());

	let db = Arc::new(
		sqlx::PgPool::connect(&database_uri)
			.await
			.expect("failed to connect to database"),
	);

	let global = Arc::new(GlobalState {
		config,
		ctx,
		requests: Mutex::new(HashMap::new()),
		nats,
		jetstream,
		db,
	});

	(global, handler)
}
