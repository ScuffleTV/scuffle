use std::sync::Arc;
use std::time::Duration;

use anyhow::Context as _;
use async_graphql::SDLExportOptions;
use binary_helper::config::RedisConfig;
use binary_helper::global::*;
use binary_helper::{bootstrap, grpc_health, grpc_server, impl_global_traits};
use platform_api::config::{ApiConfig, IgDbConfig, ImageUploaderConfig, JwtConfig, TurnstileConfig, VideoApiConfig};
use platform_api::dataloader::category::CategoryByIdLoader;
use platform_api::dataloader::global_state::GlobalStateLoader;
use platform_api::dataloader::role::RoleByIdLoader;
use platform_api::dataloader::session::SessionByIdLoader;
use platform_api::dataloader::uploaded_file::UploadedFileByIdLoader;
use platform_api::dataloader::user::{UserByIdLoader, UserByUsernameLoader};
use platform_api::subscription::SubscriptionManager;
use platform_api::video_api::{
	load_playback_keypair_private_key, setup_video_events_client, setup_video_playback_session_client,
	setup_video_room_client, VideoEventsClient, VideoPlaybackSessionClient, VideoRoomClient,
};
use platform_api::{igdb_cron, image_upload_callback, video_event_handler};
use scuffle_utils::context::Context;
use scuffle_utilsdataloader::DataLoader;
use scuffle_utilsgrpc::TlsSettings;
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

	/// The IGDB config
	igdb: IgDbConfig,

	/// Redis Config
	redis: RedisConfig,
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
	db: Arc<utils::database::Pool>,

	category_by_id_loader: DataLoader<CategoryByIdLoader>,
	global_state_loader: DataLoader<GlobalStateLoader>,
	role_by_id_loader: DataLoader<RoleByIdLoader>,
	session_by_id_loader: DataLoader<SessionByIdLoader>,
	user_by_id_loader: DataLoader<UserByIdLoader>,
	user_by_username_loader: DataLoader<UserByUsernameLoader>,
	uploader_file_by_id_loader: DataLoader<UploadedFileByIdLoader>,

	subscription_manager: SubscriptionManager,

	image_processor_s3: binary_helper::s3::Bucket,

	video_room_client: VideoRoomClient,
	video_playback_session_client: VideoPlaybackSessionClient,
	video_events_client: VideoEventsClient,

	redis: Arc<fred::clients::RedisPool>,

	playback_private_key: Option<jwt_next::asymmetric::AsymmetricKeyWithDigest<jwt_next::asymmetric::SigningKey>>,
}

impl_global_traits!(GlobalState);

impl binary_helper::global::GlobalRedis for GlobalState {
	#[inline(always)]
	fn redis(&self) -> &Arc<fred::clients::RedisPool> {
		&self.redis
	}
}

impl binary_helper::global::GlobalConfigProvider<ApiConfig> for GlobalState {
	#[inline(always)]
	fn provide_config(&self) -> &ApiConfig {
		&self.config.extra.api
	}
}

impl binary_helper::global::GlobalConfigProvider<TurnstileConfig> for GlobalState {
	#[inline(always)]
	fn provide_config(&self) -> &TurnstileConfig {
		&self.config.extra.turnstile
	}
}

impl binary_helper::global::GlobalConfigProvider<JwtConfig> for GlobalState {
	#[inline(always)]
	fn provide_config(&self) -> &JwtConfig {
		&self.config.extra.jwt
	}
}

impl binary_helper::global::GlobalConfigProvider<ImageUploaderConfig> for GlobalState {
	#[inline(always)]
	fn provide_config(&self) -> &ImageUploaderConfig {
		&self.config.extra.image_uploader
	}
}

impl binary_helper::global::GlobalConfigProvider<VideoApiConfig> for GlobalState {
	#[inline(always)]
	fn provide_config(&self) -> &VideoApiConfig {
		&self.config.extra.video_api
	}
}

impl binary_helper::global::GlobalConfigProvider<IgDbConfig> for GlobalState {
	#[inline(always)]
	fn provide_config(&self) -> &IgDbConfig {
		&self.config.extra.igdb
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

	fn image_uploader_s3(&self) -> &binary_helper::s3::Bucket {
		&self.image_processor_s3
	}

	fn video_room_client(&self) -> &VideoRoomClient {
		&self.video_room_client
	}

	fn video_playback_session_client(&self) -> &VideoPlaybackSessionClient {
		&self.video_playback_session_client
	}

	fn video_events_client(&self) -> &VideoEventsClient {
		&self.video_events_client
	}

	fn playback_private_key(
		&self,
	) -> &Option<jwt_next::asymmetric::AsymmetricKeyWithDigest<jwt_next::asymmetric::SigningKey>> {
		&self.playback_private_key
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

		let image_processor_s3 = config.extra.image_uploader.bucket.setup();

		let video_api_tls = if let Some(tls) = &config.extra.video_api.tls {
			let cert = tokio::fs::read(&tls.cert)
				.await
				.context("failed to read video api tls cert")?;
			let key = tokio::fs::read(&tls.key).await.context("failed to read video api tls key")?;

			let ca_cert = if let Some(ca_cert) = &tls.ca_cert {
				Some(tonic::transport::Certificate::from_pem(
					tokio::fs::read(&ca_cert).await.context("failed to read video api tls ca")?,
				))
			} else {
				None
			};

			Some(TlsSettings {
				domain: tls.domain.clone(),
				ca_cert,
				identity: tonic::transport::Identity::from_pem(cert, key),
			})
		} else {
			None
		};

		let video_api_channel = scuffle_utilsgrpc::make_channel(
			vec![config.extra.video_api.address.clone()],
			Duration::from_secs(30),
			video_api_tls,
		)?;
		let video_room_client = setup_video_room_client(video_api_channel.clone(), &config.extra.video_api);
		let video_playback_session_client =
			setup_video_playback_session_client(video_api_channel.clone(), &config.extra.video_api);
		let video_events_client = setup_video_events_client(video_api_channel.clone(), &config.extra.video_api);

		let playback_private_key = config
			.extra
			.video_api
			.playback_keypair
			.as_ref()
			.map(load_playback_keypair_private_key)
			.transpose()?;

		let redis = setup_redis(&config.extra.redis).await?;

		Ok(Self {
			ctx,
			config,
			nats,
			jetstream,
			db,
			redis,
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
			video_playback_session_client,
			video_events_client,
			playback_private_key,
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
		let image_upload_callback = image_upload_callback::run(global.clone());
		let igdb_cron = igdb_cron::run(global.clone());

		select! {
			r = grpc_future => r.context("grpc server stopped unexpectedly")?,
			r = api_future => r.context("api server stopped unexpectedly")?,
			r = subscription_manager => r.context("subscription manager stopped unexpectedly")?,
			r = video_event_handler => r.context("video event handler stopped unexpectedly")?,
			r = image_upload_callback => r.context("image processor callback handler stopped unexpectedly")?,
			r = igdb_cron => r.context("igdb cron stopped unexpectedly")?,
		}

		Ok(())
	})
	.await
	{
		tracing::error!("{:#}", err);
		std::process::exit(1);
	}
}
