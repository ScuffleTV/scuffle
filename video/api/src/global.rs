use common::dataloader::DataLoader;

use crate::{config::ApiConfig, dataloaders};

pub trait ApiState {
    fn access_token_loader(&self) -> &DataLoader<dataloaders::AccessTokenLoader>;
}

pub trait ApiGlobal:
    common::global::GlobalCtx
    + common::global::GlobalConfigProvider<ApiConfig>
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
        + common::global::GlobalNats
        + common::global::GlobalDb
        + common::global::GlobalConfig
        + ApiState
        + Send
        + Sync
        + 'static
{
}
