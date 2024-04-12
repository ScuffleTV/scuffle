use utils::dataloader::DataLoader;

use crate::config::{ApiConfig, IgDbConfig, ImageUploaderConfig, JwtConfig, TurnstileConfig, VideoApiConfig};
use crate::dataloader::category::CategoryByIdLoader;
use crate::dataloader::global_state::GlobalStateLoader;
use crate::dataloader::role::RoleByIdLoader;
use crate::dataloader::session::SessionByIdLoader;
use crate::dataloader::uploaded_file::UploadedFileByIdLoader;
use crate::dataloader::user::{UserByIdLoader, UserByUsernameLoader};
use crate::subscription::SubscriptionManager;
use crate::video_api::{VideoEventsClient, VideoPlaybackSessionClient, VideoRoomClient};

pub trait ApiState {
	fn user_by_username_loader(&self) -> &DataLoader<UserByUsernameLoader>;
	fn user_by_id_loader(&self) -> &DataLoader<UserByIdLoader>;
	fn session_by_id_loader(&self) -> &DataLoader<SessionByIdLoader>;
	fn role_by_id_loader(&self) -> &DataLoader<RoleByIdLoader>;
	fn category_by_id_loader(&self) -> &DataLoader<CategoryByIdLoader>;
	fn global_state_loader(&self) -> &DataLoader<GlobalStateLoader>;
	fn uploaded_file_by_id_loader(&self) -> &DataLoader<UploadedFileByIdLoader>;

	fn subscription_manager(&self) -> &SubscriptionManager;

	fn image_uploader_s3(&self) -> &binary_helper::s3::Bucket;

	fn video_room_client(&self) -> &VideoRoomClient;
	fn video_playback_session_client(&self) -> &VideoPlaybackSessionClient;
	fn video_events_client(&self) -> &VideoEventsClient;

	fn playback_private_key(
		&self,
	) -> &Option<jwt_next::asymmetric::AsymmetricKeyWithDigest<jwt_next::asymmetric::SigningKey>>;
}

pub trait ApiGlobal:
	binary_helper::global::GlobalCtx
	+ binary_helper::global::GlobalConfigProvider<ApiConfig>
	+ binary_helper::global::GlobalConfigProvider<TurnstileConfig>
	+ binary_helper::global::GlobalConfigProvider<JwtConfig>
	+ binary_helper::global::GlobalConfigProvider<ImageUploaderConfig>
	+ binary_helper::global::GlobalConfigProvider<VideoApiConfig>
	+ binary_helper::global::GlobalConfigProvider<IgDbConfig>
	+ binary_helper::global::GlobalNats
	+ binary_helper::global::GlobalDb
	+ binary_helper::global::GlobalRedis
	+ binary_helper::global::GlobalConfig
	+ ApiState
	+ Send
	+ Sync
	+ 'static
{
}

impl<T> ApiGlobal for T where
	T: binary_helper::global::GlobalCtx
		+ binary_helper::global::GlobalConfigProvider<ApiConfig>
		+ binary_helper::global::GlobalConfigProvider<TurnstileConfig>
		+ binary_helper::global::GlobalConfigProvider<JwtConfig>
		+ binary_helper::global::GlobalConfigProvider<ImageUploaderConfig>
		+ binary_helper::global::GlobalConfigProvider<VideoApiConfig>
		+ binary_helper::global::GlobalConfigProvider<IgDbConfig>
		+ binary_helper::global::GlobalNats
		+ binary_helper::global::GlobalDb
		+ binary_helper::global::GlobalRedis
		+ binary_helper::global::GlobalConfig
		+ ApiState
		+ Send
		+ Sync
		+ 'static
{
}
