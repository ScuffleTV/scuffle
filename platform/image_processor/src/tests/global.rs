use std::sync::Arc;

use common::context::Context;

use crate::config::ImageProcessorConfig;

pub struct GlobalState {
	ctx: Context,
	config: ImageProcessorConfig,
	nats: async_nats::Client,
	jetstream: async_nats::jetstream::Context,
	db: Arc<sqlx::PgPool>,
	s3_source_bucket: s3::Bucket,
	s3_target_bucket: s3::Bucket,
}

impl common::global::GlobalCtx for GlobalState {
	fn ctx(&self) -> &Context {
		&self.ctx
	}
}

impl common::global::GlobalConfigProvider<ImageProcessorConfig> for GlobalState {
	fn provide_config(&self) -> &ImageProcessorConfig {
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

impl crate::global::ImageProcessorState for GlobalState {
	fn s3_source_bucket(&self) -> &s3::Bucket {
		&self.s3_source_bucket
	}

	fn s3_target_bucket(&self) -> &s3::Bucket {
		&self.s3_target_bucket
	}
}

// pub async fn mock_global_state(config: ImageProcessorConfig) ->
// (Arc<GlobalState>, Handler) { 	let (ctx, handler) = Context::new();

// 	dotenvy::dotenv().ok();

// 	let logging_level = std::env::var("LOGGING_LEVEL").unwrap_or_else(|_|
// "info".to_string());

// 	logging::init(&logging_level, Default::default()).expect("failed to
// initialize logging");

// 	let database_uri =
// std::env::var("PLATFORM_DATABASE_URL_TEST").expect("
// PLATFORM_DATABASE_URL_TEST must be set"); 	let nats_addr =
// std::env::var("NATS_ADDR").expect("NATS_URL must be set");

// 	let nats = async_nats::connect(&nats_addr).await.expect("failed to connect to
// nats"); 	let jetstream = async_nats::jetstream::new(nats.clone());

// 	let db = Arc::new(
// 		sqlx::PgPool::connect(&database_uri)
// 			.await
// 			.expect("failed to connect to database"),
// 	);

// 	let global = Arc::new(GlobalState {
// 		s3_source_bucket: config.source_bucket.setup().await.expect("failed to setup
// source bucket"), 		s3_target_bucket:
// config.target_bucket.setup().await.expect("failed to setup target bucket"),
// 		config,
// 		ctx,
// 		nats,
// 		jetstream,
// 		db,
// 	});

// 	(global, handler)
// }
