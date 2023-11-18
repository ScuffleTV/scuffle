use std::sync::Arc;

use anyhow::Context as _;
use binary_helper::global::{setup_database, setup_nats};
use binary_helper::{bootstrap, grpc_health, grpc_server, impl_global_traits};
use common::context::Context;
use common::global::{GlobalCtx, GlobalDb, GlobalNats};
use tokio::select;
use video_edge::config::EdgeConfig;
use video_edge::subscription;

#[derive(Debug, Clone, Default, serde::Deserialize, config::Config)]
#[serde(default)]
struct ExtConfig {
	/// The Edge configuration.
	edge: EdgeConfig,
}

impl binary_helper::config::ConfigExtention for ExtConfig {
	const APP_NAME: &'static str = "video-edge";
}

type AppConfig = binary_helper::config::AppConfig<ExtConfig>;

struct GlobalState {
	ctx: Context,
	config: AppConfig,
	nats: async_nats::Client,
	jetstream: async_nats::jetstream::Context,
	db: Arc<sqlx::PgPool>,
	metadata_store: async_nats::jetstream::kv::Store,
	media_store: async_nats::jetstream::object_store::ObjectStore,
	subscriber: subscription::SubscriptionManager,
}

impl_global_traits!(GlobalState);

impl common::global::GlobalConfigProvider<EdgeConfig> for GlobalState {
	#[inline(always)]
	fn provide_config(&self) -> &EdgeConfig {
		&self.config.extra.edge
	}
}

impl video_edge::global::EdgeState for GlobalState {
	#[inline(always)]
	fn metadata_store(&self) -> &async_nats::jetstream::kv::Store {
		&self.metadata_store
	}

	#[inline(always)]
	fn media_store(&self) -> &async_nats::jetstream::object_store::ObjectStore {
		&self.media_store
	}

	#[inline(always)]
	fn subscriber(&self) -> &subscription::SubscriptionManager {
		&self.subscriber
	}
}

#[async_trait::async_trait]
impl binary_helper::Global<AppConfig> for GlobalState {
	async fn new(ctx: Context, config: AppConfig) -> anyhow::Result<Self> {
		let (nats, jetstream) = setup_nats(&config.name, &config.nats).await?;
		let db = setup_database(&config.database).await?;

		let metadata_store = jetstream
			.get_key_value(&config.extra.edge.metadata_kv_store)
			.await
			.context("failed to get metadata store")?;
		let media_store = jetstream
			.get_object_store(&config.extra.edge.media_ob_store)
			.await
			.context("failed to get media store")?;

		Ok(Self {
			ctx,
			config,
			nats,
			jetstream,
			db,
			metadata_store,
			media_store,
			subscriber: subscription::SubscriptionManager::default(),
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

			let router = video_edge::grpc::add_routes(&global, router);

			router.serve_with_shutdown(global.config.grpc.bind_address, async {
				global.ctx().done().await;
			})
		};

		let edge_future = video_edge::edge::run(global.clone());

		select! {
			r = grpc_future => r.context("grpc server stopped unexpectedly")?,
			r = edge_future => r.context("edge server stopped unexpectedly")?,
		}

		Ok(())
	})
	.await
	{
		tracing::error!("{:#}", err);
		std::process::exit(1);
	}
}
