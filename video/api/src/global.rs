use common::dataloader::DataLoader;

use crate::config::ApiConfig;
use crate::dataloaders;

pub trait ApiState {
	fn access_token_loader(&self) -> &DataLoader<dataloaders::AccessTokenLoader>;
	fn recording_state_loader(&self) -> &DataLoader<dataloaders::RecordingStateLoader>;
	fn room_loader(&self) -> &DataLoader<dataloaders::RoomLoader>;
	fn events_stream(&self) -> &async_nats::jetstream::stream::Stream;
}

pub trait ApiGlobal:
	common::global::GlobalCtx
	+ common::global::GlobalConfigProvider<ApiConfig>
	+ common::global::GlobalNats
	+ common::global::GlobalDb
	+ common::global::GlobalConfig
	+ common::global::GlobalRedis
	+ ApiState
	+ Send
	+ Sync
	+ 'static
{
}

impl<T> ApiGlobal for T where
	T: common::global::GlobalCtx
		+ common::global::GlobalConfigProvider<ApiConfig>
		+ common::global::GlobalNats
		+ common::global::GlobalDb
		+ common::global::GlobalConfig
		+ common::global::GlobalRedis
		+ ApiState
		+ Send
		+ Sync
		+ 'static
{
}
