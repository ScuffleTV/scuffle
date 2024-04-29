#![allow(dead_code)]

use std::sync::Arc;

use anyhow::Context as _;
use binary_helper::global::{setup_database, setup_nats, GlobalCtx, GlobalDb, GlobalNats};
use binary_helper::{bootstrap, grpc_health, grpc_server, impl_global_traits};
use platform_image_processor::config::ImageProcessorConfig;
use scuffle_utils::context::Context;
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

pub fn main() {
	tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.max_blocking_threads(
			std::env::var("TOKIO_MAX_BLOCKING_THREADS")
				.ok()
				.and_then(|v| v.parse().ok())
				.unwrap_or(2048),
		)
		.build()
		.expect("failed to create tokio runtime")
		.block_on(async {
			if let Err(err) = bootstrap::<AppConfig, GlobalState, _>(|global| async move {
				let grpc_future = {
					let mut server = grpc_server(&global.config.grpc)
						.await
						.context("failed to create grpc server")?;
					let router = server.add_service(grpc_health::HealthServer::new(&global, |global, _| async move {
						!global.db().is_closed()
							&& global.nats().connection_state() == async_nats::connection::State::Connected
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
		})
}
