use std::io;
use std::sync::Arc;
use std::time::Duration;

use common::context::Context;
use common::dataloader::DataLoader;

use crate::config::AppConfig;
use crate::dataloader::category::CategorySearchLoader;
use crate::dataloader::user::UserSearchLoader;
use crate::dataloader::{
    category::CategoryByIdLoader, global_state::GlobalStateLoader, role::RoleByIdLoader,
    session::SessionByIdLoader, user::UserByIdLoader, user::UserByUsernameLoader,
};
use crate::subscription::SubscriptionManager;

pub mod turnstile;

pub struct GlobalState {
    pub config: AppConfig,
    pub db: Arc<sqlx::PgPool>,
    pub ctx: Context,

    pub user_by_username_loader: DataLoader<UserByUsernameLoader>,
    pub user_by_id_loader: DataLoader<UserByIdLoader>,
    pub user_search_loader: DataLoader<UserSearchLoader>,
    pub session_by_id_loader: DataLoader<SessionByIdLoader>,
    pub role_by_id_loader: DataLoader<RoleByIdLoader>,
    pub category_by_id_loader: DataLoader<CategoryByIdLoader>,
    pub category_search_loader: DataLoader<CategorySearchLoader>,
    pub global_state_loader: DataLoader<GlobalStateLoader>,

    pub subscription_manager: SubscriptionManager,
    pub nats: async_nats::Client,
}

impl GlobalState {
    pub fn new(
        config: AppConfig,
        db: Arc<sqlx::PgPool>,
        nats: async_nats::Client,
        ctx: Context,
    ) -> Self {
        Self {
            config,
            ctx,

            user_by_username_loader: UserByUsernameLoader::new(db.clone()),
            user_by_id_loader: UserByIdLoader::new(db.clone()),
            user_search_loader: UserSearchLoader::new(db.clone()),
            session_by_id_loader: SessionByIdLoader::new(db.clone()),
            role_by_id_loader: RoleByIdLoader::new(db.clone()),
            category_by_id_loader: CategoryByIdLoader::new(db.clone()),
            category_search_loader: CategorySearchLoader::new(db.clone()),
            global_state_loader: GlobalStateLoader::new(db.clone()),

            subscription_manager: SubscriptionManager::default(),
            db,
            nats,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum SetupNatsError {
    #[error("failed to parse address: {0}")]
    AddressParse(io::Error),
    #[error("connect error: {0}")]
    ConnectError(#[from] async_nats::ConnectError),
}

pub async fn setup_nats(config: &AppConfig) -> Result<async_nats::Client, SetupNatsError> {
    let mut options = async_nats::ConnectOptions::new()
        .connection_timeout(Duration::from_secs(5))
        .name(&config.name)
        .retry_on_initial_connect();

    if let Some(user) = &config.nats.username {
        options = options.user_and_password(
            user.clone(),
            config.nats.password.clone().unwrap_or_default(),
        )
    } else if let Some(token) = &config.nats.token {
        options = options.token(token.clone())
    }

    if let Some(tls) = &config.nats.tls {
        options = options
            .require_tls(true)
            .add_root_certificates((&tls.ca_cert).into())
            .add_client_certificate((&tls.cert).into(), (&tls.key).into());
    }

    let nats_addrs = config
        .nats
        .servers
        .iter()
        .map(|s| s.parse::<async_nats::ServerAddr>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(SetupNatsError::AddressParse)?;

    let nats = options.connect(nats_addrs).await?;

    tracing::info!("connected to nats");

    Ok(nats)
}
