use std::sync::Arc;

use anyhow::Context as _;
use async_graphql::SDLExportOptions;
use binary_helper::{
    bootstrap,
    global::{setup_database, setup_nats},
    grpc_health, grpc_server, impl_global_traits,
};
use common::{context::Context, dataloader::DataLoader, global::*};
use platform_api::{
    config::{ApiConfig, JwtConfig, TurnstileConfig},
    dataloader::{
        category::{CategoryByIdLoader, CategorySearchLoader},
        global_state::GlobalStateLoader,
        role::RoleByIdLoader,
        session::SessionByIdLoader,
        user::{UserByIdLoader, UserByUsernameLoader, UserSearchLoader},
    },
    subscription::SubscriptionManager,
};
use tokio::select;

#[derive(Debug, Clone, Default, config::Config, serde::Deserialize)]
#[serde(default)]
/// The API is the backend for the Scuffle service
struct ExtConfig {
    /// If we should export the GraphQL schema, if set to true, the schema will be exported to the stdout, and the program will exit.
    export_gql: bool,

    /// API Config
    api: ApiConfig,

    /// Turnstile Config
    turnstile: TurnstileConfig,

    /// JWT Config
    jwt: JwtConfig,
}

impl binary_helper::config::ConfigExtention for ExtConfig {
    const APP_NAME: &'static str = "scuffle-api";

    fn pre_hook(config: &mut AppConfig) -> anyhow::Result<()> {
        if config.extra.export_gql {
            let schema = platform_api::api::v1::gql::schema::<GlobalState>();

            println!(
                "{}",
                schema.sdl_with_options(
                    SDLExportOptions::default()
                        .federation()
                        .include_specified_by()
                        .sorted_arguments()
                        .sorted_enum_items()
                        .sorted_fields()
                )
            );
            std::process::exit(0);
        }

        Ok(())
    }
}

type AppConfig = binary_helper::config::AppConfig<ExtConfig>;

struct GlobalState {
    ctx: Context,
    config: AppConfig,
    nats: async_nats::Client,
    jetstream: async_nats::jetstream::Context,
    db: Arc<sqlx::PgPool>,

    category_by_id_loader: DataLoader<CategoryByIdLoader>,
    category_search_loader: DataLoader<CategorySearchLoader>,
    global_state_loader: DataLoader<GlobalStateLoader>,
    role_by_id_loader: DataLoader<RoleByIdLoader>,
    session_by_id_loader: DataLoader<SessionByIdLoader>,
    user_by_id_loader: DataLoader<UserByIdLoader>,
    user_by_username_loader: DataLoader<UserByUsernameLoader>,
    user_search_loader: DataLoader<UserSearchLoader>,

    subscription_manager: SubscriptionManager,
}

impl_global_traits!(GlobalState);

impl common::global::GlobalConfigProvider<ApiConfig> for GlobalState {
    #[inline(always)]
    fn provide_config(&self) -> &ApiConfig {
        &self.config.extra.api
    }
}

impl common::global::GlobalConfigProvider<TurnstileConfig> for GlobalState {
    #[inline(always)]
    fn provide_config(&self) -> &TurnstileConfig {
        &self.config.extra.turnstile
    }
}

impl common::global::GlobalConfigProvider<JwtConfig> for GlobalState {
    #[inline(always)]
    fn provide_config(&self) -> &JwtConfig {
        &self.config.extra.jwt
    }
}

impl platform_api::global::ApiState for GlobalState {
    fn category_by_id_loader(&self) -> &DataLoader<CategoryByIdLoader> {
        &self.category_by_id_loader
    }

    fn category_search_loader(&self) -> &DataLoader<CategorySearchLoader> {
        &self.category_search_loader
    }

    fn global_state_loader(&self) -> &DataLoader<GlobalStateLoader> {
        &self.global_state_loader
    }

    fn role_by_id_loader(&self) -> &DataLoader<RoleByIdLoader> {
        &self.role_by_id_loader
    }

    fn session_by_id_loader(&self) -> &DataLoader<SessionByIdLoader> {
        &self.session_by_id_loader
    }

    fn user_by_id_loader(&self) -> &DataLoader<UserByIdLoader> {
        &self.user_by_id_loader
    }

    fn user_by_username_loader(&self) -> &DataLoader<UserByUsernameLoader> {
        &self.user_by_username_loader
    }

    fn user_search_loader(&self) -> &DataLoader<UserSearchLoader> {
        &self.user_search_loader
    }

    fn subscription_manager(&self) -> &SubscriptionManager {
        &self.subscription_manager
    }
}

#[async_trait::async_trait]
impl binary_helper::Global<AppConfig> for GlobalState {
    async fn new(ctx: Context, config: AppConfig) -> anyhow::Result<Self> {
        let (nats, jetstream) = setup_nats(&config.name, &config.nats).await?;
        let db = setup_database(&config.database).await?;

        let category_by_id_loader = CategoryByIdLoader::new(db.clone());
        let category_search_loader = CategorySearchLoader::new(db.clone());
        let global_state_loader = GlobalStateLoader::new(db.clone());
        let role_by_id_loader = RoleByIdLoader::new(db.clone());
        let session_by_id_loader = SessionByIdLoader::new(db.clone());
        let user_by_id_loader = UserByIdLoader::new(db.clone());
        let user_by_username_loader = UserByUsernameLoader::new(db.clone());
        let user_search_loader = UserSearchLoader::new(db.clone());

        let subscription_manager = SubscriptionManager::default();

        Ok(Self {
            ctx,
            config,
            nats,
            jetstream,
            db,
            category_by_id_loader,
            category_search_loader,
            global_state_loader,
            role_by_id_loader,
            session_by_id_loader,
            user_by_id_loader,
            user_by_username_loader,
            user_search_loader,
            subscription_manager,
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

            let router = platform_api::grpc::add_routes(&global, router);

            router.serve_with_shutdown(global.config.grpc.bind_address, async {
                global.ctx().done().await;
            })
        };

        let api_future = platform_api::api::run(global.clone());

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
