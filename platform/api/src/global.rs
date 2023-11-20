use common::dataloader::DataLoader;

use crate::config::{ApiConfig, JwtConfig, TurnstileConfig};
use crate::dataloader::category::CategoryByIdLoader;
use crate::dataloader::global_state::GlobalStateLoader;
use crate::dataloader::role::RoleByIdLoader;
use crate::dataloader::session::SessionByIdLoader;
use crate::dataloader::user::{UserByIdLoader, UserByUsernameLoader};
use crate::subscription::SubscriptionManager;

pub trait ApiState {
	fn user_by_username_loader(&self) -> &DataLoader<UserByUsernameLoader>;
	fn user_by_id_loader(&self) -> &DataLoader<UserByIdLoader>;
	fn session_by_id_loader(&self) -> &DataLoader<SessionByIdLoader>;
	fn role_by_id_loader(&self) -> &DataLoader<RoleByIdLoader>;
	fn category_by_id_loader(&self) -> &DataLoader<CategoryByIdLoader>;
	fn global_state_loader(&self) -> &DataLoader<GlobalStateLoader>;

	fn subscription_manager(&self) -> &SubscriptionManager;
}

pub trait ApiGlobal:
	common::global::GlobalCtx
	+ common::global::GlobalConfigProvider<ApiConfig>
	+ common::global::GlobalConfigProvider<TurnstileConfig>
	+ common::global::GlobalConfigProvider<JwtConfig>
	+ common::global::GlobalNats
	+ common::global::GlobalDb
	+ common::global::GlobalConfig
	+ ApiState
	+ Send
	+ Sync
	+ 'static
{
}

impl<T> ApiGlobal for T where
	T: common::global::GlobalCtx
		+ common::global::GlobalConfigProvider<ApiConfig>
		+ common::global::GlobalConfigProvider<TurnstileConfig>
		+ common::global::GlobalConfigProvider<JwtConfig>
		+ common::global::GlobalNats
		+ common::global::GlobalDb
		+ common::global::GlobalConfig
		+ ApiState
		+ Send
		+ Sync
		+ 'static
{
}
