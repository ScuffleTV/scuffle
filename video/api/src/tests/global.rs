use std::sync::Arc;

use async_nats::jetstream::stream::{self, RetentionPolicy};
use common::context::{Context, Handler};
use common::dataloader::DataLoader;
use common::logging;
use common::prelude::FutureTimeout;
use fred::interfaces::ClientLike;

use crate::config::ApiConfig;
use crate::dataloaders;

pub struct GlobalState {
	ctx: Context,
	config: ApiConfig,
	nats: async_nats::Client,
	jetstream: async_nats::jetstream::Context,
	db: Arc<sqlx::PgPool>,
	redis: Arc<fred::clients::RedisPool>,

	access_token_loader: DataLoader<dataloaders::AccessTokenLoader>,
	recording_state_loader: DataLoader<dataloaders::RecordingStateLoader>,
	room_loader: DataLoader<dataloaders::RoomLoader>,

	events_stream: async_nats::jetstream::stream::Stream,
}

impl common::global::GlobalCtx for GlobalState {
	fn ctx(&self) -> &Context {
		&self.ctx
	}
}

impl common::global::GlobalConfigProvider<ApiConfig> for GlobalState {
	fn provide_config(&self) -> &ApiConfig {
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

impl common::global::GlobalRedis for GlobalState {
	fn redis(&self) -> &Arc<fred::clients::RedisPool> {
		&self.redis
	}
}

impl common::global::GlobalConfig for GlobalState {}

impl crate::global::ApiState for GlobalState {
	fn access_token_loader(&self) -> &DataLoader<dataloaders::AccessTokenLoader> {
		&self.access_token_loader
	}

	fn recording_state_loader(&self) -> &DataLoader<dataloaders::RecordingStateLoader> {
		&self.recording_state_loader
	}

	fn room_loader(&self) -> &DataLoader<dataloaders::RoomLoader> {
		&self.room_loader
	}

	fn events_stream(&self) -> &async_nats::jetstream::stream::Stream {
		&self.events_stream
	}
}

pub async fn mock_global_state(config: ApiConfig) -> (Arc<GlobalState>, Handler) {
	let (ctx, handler) = Context::new();

	dotenvy::dotenv().ok();

	let logging_level = std::env::var("LOGGING_LEVEL").unwrap_or_else(|_| "info".to_string());

	logging::init(&logging_level, Default::default()).expect("failed to initialize logging");

	let database_uri = std::env::var("VIDEO_DATABASE_URL_TEST").expect("VIDEO_DATABASE_URL_TEST must be set");
	let nats_addr = std::env::var("NATS_ADDR").expect("NATS_URL must be set");
	let redis_url = std::env::var("REDIS_ADDR")
		.map(|addr| format!("redis://{addr}"))
		.unwrap_or_else(|_| std::env::var("REDIS_URL").expect("REDIS_URL and REDIS_ADDR are not set"));

	let nats = async_nats::connect(&nats_addr).await.expect("failed to connect to nats");
	let jetstream = async_nats::jetstream::new(nats.clone());

	let db = Arc::new(
		sqlx::PgPool::connect(&database_uri)
			.await
			.expect("failed to connect to database"),
	);

	let redis = Arc::new(
		fred::clients::RedisPool::new(fred::types::RedisConfig::from_url(&redis_url).unwrap(), None, None, None, 1).unwrap(),
	);

	redis.connect();

	redis
		.wait_for_connect()
		.timeout(std::time::Duration::from_secs(5))
		.await
		.expect("failed to connect to redis")
		.expect("failed to connect to redis");

	common::ratelimiter::load_rate_limiter_script(&*redis)
		.await
		.expect("failed to load rate limiter script");

	let events_stream = jetstream
		.get_or_create_stream(stream::Config {
			name: config.events.stream_name.clone(),
			subjects: vec![format!("{}.>", config.events.stream_name)],
			retention: RetentionPolicy::WorkQueue,
			max_age: config.events.nats_stream_message_max_age,
			..Default::default()
		})
		.await
		.expect("failed to create event stream");

	let access_token_loader = dataloaders::AccessTokenLoader::new(db.clone());
	let recording_state_loader = dataloaders::RecordingStateLoader::new(db.clone());
	let room_loader = dataloaders::RoomLoader::new(db.clone());

	let global = Arc::new(GlobalState {
		config,
		ctx,
		nats,
		jetstream,
		db,
		access_token_loader,
		recording_state_loader,
		room_loader,
		redis,
		events_stream,
	});

	(global, handler)
}
