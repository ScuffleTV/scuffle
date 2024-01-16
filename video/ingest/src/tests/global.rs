use std::collections::HashMap;
use std::sync::Arc;

use common::context::{Context, Handler};
use common::database::deadpool_postgres::{ManagerConfig, PoolConfig, RecyclingMethod, Runtime};
use common::database::Pool;
use common::logging;
use postgres_from_row::tokio_postgres::NoTls;
use tokio::sync::{mpsc, Mutex};
use ulid::Ulid;

use crate::config::IngestConfig;
use crate::global::IncomingTranscoder;

pub struct GlobalState {
	ctx: Context,
	config: IngestConfig,
	nats: async_nats::Client,
	jetstream: async_nats::jetstream::Context,
	db: Arc<common::database::Pool>,
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
	fn db(&self) -> &Arc<common::database::Pool> {
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

	let database_uri = std::env::var("VIDEO_DATABASE_URL_TEST").expect("VIDEO_DATABASE_URL_TEST must be set");
	let nats_addr = std::env::var("NATS_ADDR").expect("NATS_URL must be set");

	let nats = async_nats::connect(&nats_addr).await.expect("failed to connect to nats");
	let jetstream = async_nats::jetstream::new(nats.clone());

	let db = Arc::new(
		Pool::builder(common::database::deadpool_postgres::Manager::from_config(
			database_uri.parse().unwrap(),
			NoTls,
			ManagerConfig {
				recycling_method: RecyclingMethod::Fast,
			},
		))
		.config(PoolConfig::default())
		.runtime(Runtime::Tokio1)
		.build()
		.expect("failed to create pool"),
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
