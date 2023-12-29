use std::sync::Arc;

use anyhow::Context as _;
use async_graphql::SDLExportOptions;
use binary_helper::global::{setup_database, setup_nats};
use binary_helper::{bootstrap, grpc_health, grpc_server, impl_global_traits};
use common::context::Context;
use common::dataloader::DataLoader;
use common::global::*;
use platform_api::config::{ApiConfig, VideoApiConfig, JwtConfig, TurnstileConfig};
use platform_api::dataloader::category::CategoryByIdLoader;
use platform_api::dataloader::global_state::GlobalStateLoader;
use platform_api::dataloader::role::RoleByIdLoader;
use platform_api::dataloader::session::SessionByIdLoader;
use platform_api::dataloader::uploaded_file::UploadedFileByIdLoader;
use platform_api::dataloader::user::{UserByIdLoader, UserByUsernameLoader};
use platform_api::subscription::SubscriptionManager;
use platform_api::video_api::{setup_video_room_client, VideoRoomClient, setup_video_events_client, VideoEventsClient};
use platform_api::video_event_handler;
use tokio::select;

#[derive(Debug, Clone, Default, config::Config, serde::Deserialize)]
#[serde(default)]
/// The API is the backend for the Scuffle service
struct ExtConfig {
	/// If we should export the GraphQL schema, if set to true, the schema will
	/// be exported to the stdout, and the program will exit.
	export_gql: bool,

	/// API Config
	api: ApiConfig,

	/// Turnstile Config
	turnstile: TurnstileConfig,

	/// JWT Config
	jwt: JwtConfig,

	/// Image Uploader Config
	image_uploader: ImageUploaderConfig,

	/// The video api config
	video_api: VideoApiConfig,
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
	global_state_loader: DataLoader<GlobalStateLoader>,
	role_by_id_loader: DataLoader<RoleByIdLoader>,
	session_by_id_loader: DataLoader<SessionByIdLoader>,
	user_by_id_loader: DataLoader<UserByIdLoader>,
	user_by_username_loader: DataLoader<UserByUsernameLoader>,
	uploader_file_by_id_loader: DataLoader<UploadedFileByIdLoader>,

	subscription_manager: SubscriptionManager,

	image_processor_s3: s3::Bucket,

	video_room_client: VideoRoomClient,
	video_events_client: VideoEventsClient,
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

impl common::global::GlobalConfigProvider<ImageUploaderConfig> for GlobalState {
	#[inline(always)]
	fn provide_config(&self) -> &ImageUploaderConfig {
		&self.config.extra.image_uploader
	}
}

impl platform_api::global::ApiState for GlobalState {
	fn category_by_id_loader(&self) -> &DataLoader<CategoryByIdLoader> {
		&self.category_by_id_loader
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

	fn uploaded_file_by_id_loader(&self) -> &DataLoader<UploadedFileByIdLoader> {
		&self.uploader_file_by_id_loader
	}

	fn subscription_manager(&self) -> &SubscriptionManager {
		&self.subscription_manager
	}

	fn image_uploader_s3(&self) -> &s3::Bucket {
		&self.image_processor_s3
	}

	fn video_room_client(&self) -> &VideoRoomClient {
		&self.video_room_client
	}

	fn video_events_client(&self) -> &VideoEventsClient {
		&self.video_events_client
	}
}

impl binary_helper::Global<AppConfig> for GlobalState {
	async fn new(ctx: Context, config: AppConfig) -> anyhow::Result<Self> {
		let (nats, jetstream) = setup_nats(&config.name, &config.nats).await?;
		let db = setup_database(&config.database).await?;

		let category_by_id_loader = CategoryByIdLoader::new(db.clone());
		let global_state_loader = GlobalStateLoader::new(db.clone());
		let role_by_id_loader = RoleByIdLoader::new(db.clone());
		let session_by_id_loader = SessionByIdLoader::new(db.clone());
		let user_by_id_loader = UserByIdLoader::new(db.clone());
		let user_by_username_loader = UserByUsernameLoader::new(db.clone());
		let uploader_file_by_id_loader = UploadedFileByIdLoader::new(db.clone());

		let subscription_manager = SubscriptionManager::default();


		let image_processor_s3 = config
			.extra
			.image_uploader
			.bucket
			.setup()
			.ok_or_else(|| anyhow::anyhow!("failed to setup image processor s3"))?;

		let video_room_client = setup_video_room_client(&config.extra.video_api)?;
		let video_events_client = setup_video_events_client(&config.extra.video_api)?;

		Ok(Self {
			ctx,
			config,
			nats,
			jetstream,
			db,
			category_by_id_loader,
			global_state_loader,
			role_by_id_loader,
			session_by_id_loader,
			user_by_id_loader,
			user_by_username_loader,
			uploader_file_by_id_loader,
			subscription_manager,
			image_processor_s3,
			video_room_client,
			video_events_client,
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

			let router = platform_api::grpc::add_routes(&global, router);

			router.serve_with_shutdown(global.config.grpc.bind_address, async {
				global.ctx().done().await;
			})
		};

		let api_future = platform_api::api::run(global.clone());

		let subscription_manager = global.subscription_manager.run(global.ctx.clone(), global.nats.clone());

		let video_event_handler = video_event_handler::run(global.clone());

		select! {
			r = grpc_future => r.context("grpc server stopped unexpectedly")?,
			r = api_future => r.context("api server stopped unexpectedly")?,
			r = subscription_manager => r.context("subscription manager stopped unexpectedly")?,
			r = video_event_handler => r.context("video event handler stopped unexpectedly")?,
		}

		Ok(())
	})
	.await
	{
		tracing::error!("{:#}", err);
		std::process::exit(1);
	}
}
