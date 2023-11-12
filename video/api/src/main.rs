use std::sync::Arc;

use anyhow::Context as _;
use binary_helper::{
    bootstrap,
    global::{setup_database, setup_nats},
    grpc_health, grpc_server, impl_global_traits,
};
use common::{
    context::Context,
    dataloader::DataLoader,
    global::{GlobalCtx, GlobalDb, GlobalNats},
};
use tokio::select;
use video_api::{config::ApiConfig, dataloaders};

#[derive(Debug, Clone, Default, serde::Deserialize, config::Config)]
#[serde(default)]
struct ExtConfig {
    /// The API configuration.
    api: ApiConfig,
}

impl binary_helper::config::ConfigExtention for ExtConfig {
    const APP_NAME: &'static str = "video-api";
}

type AppConfig = binary_helper::config::AppConfig<ExtConfig>;

struct GlobalState {
    ctx: Context,
    config: AppConfig,
    nats: async_nats::Client,
    jetstream: async_nats::jetstream::Context,
    db: Arc<sqlx::PgPool>,
    access_token_loader: DataLoader<dataloaders::AccessTokenLoader>,
}

impl_global_traits!(GlobalState);

impl common::global::GlobalConfigProvider<ApiConfig> for GlobalState {
    #[inline(always)]
    fn provide_config(&self) -> &ApiConfig {
        &self.config.extra.api
    }
}

impl video_api::global::ApiState for GlobalState {
    #[inline(always)]
    fn access_token_loader(&self) -> &DataLoader<dataloaders::AccessTokenLoader> {
        &self.access_token_loader
    }
}

#[async_trait::async_trait]
impl binary_helper::Global<AppConfig> for GlobalState {
    async fn new(ctx: Context, config: AppConfig) -> anyhow::Result<Self> {
        let (nats, jetstream) = setup_nats(&config.name, &config.nats).await?;
        let db = setup_database(&config.database).await?;

        let access_token_loader = dataloaders::AccessTokenLoader::new(db.clone());

        Ok(Self {
            ctx,
            config,
            nats,
            jetstream,
            db,
            access_token_loader,
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
            let router = server.add_service(grpc_health::HealthServer::new(
                &global,
                |global, _| async move {
                    !global.db().is_closed()
                        && global.nats().connection_state()
                            == async_nats::connection::State::Connected
                },
            ));

            let router = video_api::grpc::add_routes(&global, router);

            router.serve_with_shutdown(global.config.grpc.bind_address, async {
                global.ctx().done().await;
            })
        };

        let api_future = video_api::api::run(global.clone());

        select! {
            r = grpc_future => r.context("grpc server stopped unexpectedly")?,
            r = api_future => r.context("api server stopped unexpectedly")?,
        }

        Ok(())
    })
    .await
    {
        tracing::error!("{:#}", err);
        std::process::exit(1);
    }
}
