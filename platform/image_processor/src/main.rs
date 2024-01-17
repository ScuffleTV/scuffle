use std::sync::Arc;

use anyhow::Context as _;
use binary_helper::global::{setup_database, setup_nats};
use binary_helper::{bootstrap, grpc_health, grpc_server, impl_global_traits};
use common::context::Context;
use common::global::{GlobalCtx, GlobalDb, GlobalNats};
use platform_image_processor::config::ImageProcessorConfig;
use tokio::select;

#[derive(Debug, Clone, Default, config::Config, serde::Deserialize)]
#[serde(default)]
struct ExtConfig {
	image_processor: ImageProcessorConfig,
}

impl binary_helper::config::ConfigExtention for ExtConfig {
	const APP_NAME: &'static str = "scuffle-image-processor";
}

// TODO: We don't need grpc and nats
type AppConfig = binary_helper::config::AppConfig<ExtConfig>;

struct GlobalState {
	ctx: Context,
	db: Arc<sqlx::PgPool>,
	config: AppConfig,
	nats: async_nats::Client,
	jetstream: async_nats::jetstream::Context,
	s3_source_bucket: common::s3::Bucket,
	s3_target_bucket: common::s3::Bucket,
}

impl_global_traits!(GlobalState);

impl common::global::GlobalConfigProvider<ImageProcessorConfig> for GlobalState {
	#[inline(always)]
	fn provide_config(&self) -> &ImageProcessorConfig {
		&self.config.extra.image_processor
	}
}

impl platform_image_processor::global::ImageProcessorState for GlobalState {
	#[inline(always)]
	fn s3_source_bucket(&self) -> &common::s3::Bucket {
		&self.s3_source_bucket
	}

	#[inline(always)]
	fn s3_target_bucket(&self) -> &common::s3::Bucket {
		&self.s3_target_bucket
	}
}

impl binary_helper::Global<AppConfig> for GlobalState {
	async fn new(ctx: Context, config: AppConfig) -> anyhow::Result<Self> {
		let db = setup_database(&config.database).await?;
		let s3_source_bucket = config.extra.image_processor.source_bucket.setup();
		let s3_target_bucket = config.extra.image_processor.target_bucket.setup();

		let (nats, jetstream) = setup_nats(&config.name, &config.nats).await?;

		Ok(Self {
			ctx,
			db,
			nats,
			jetstream,
			config,
			s3_source_bucket,
			s3_target_bucket,
		})
	}
}

#[tokio::main]
pub async fn main() {
	if let Err(err) = bootstrap::<AppConfig, GlobalState, _>(|global| async move {
		let grpc_future = {
			let mut server = grpc_server(&global.config.grpc)
				.await
				.context("failed to create grpc server")?;
			let router = server.add_service(grpc_health::HealthServer::new(&global, |global, _| async move {
				!global.db().is_closed() && global.nats().connection_state() == async_nats::connection::State::Connected
			}));

			let router = platform_image_processor::grpc::add_routes(&global, router);

			router.serve_with_shutdown(global.config.grpc.bind_address, async {
				global.ctx().done().await;
			})
		};

		let processor_future = platform_image_processor::processor::run(global.clone());

		select! {
			r = grpc_future => r.context("grpc server stopped unexpectedly")?,
			r = processor_future => r.context("processor stopped unexpectedly")?,
		}

		Ok(())
	})
	.await
	{
		tracing::error!("{:#}", err);
		std::process::exit(1);
	}
}
