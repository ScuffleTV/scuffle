use std::sync::Arc;

use async_graphql::dataloader::DataLoader;
use common::context::Context;

use crate::config::AppConfig;
use crate::dataloader::{
    session::SessionByIdLoader, user::UserByIdLoader, user::UserByUsernameLoader,
};

pub mod turnstile;

pub struct GlobalState {
    pub config: AppConfig,
    pub db: Arc<sqlx::PgPool>,
    pub ctx: Context,
    pub user_by_username_loader: DataLoader<UserByUsernameLoader>,
    pub user_by_id_loader: DataLoader<UserByIdLoader>,
    pub session_by_id_loader: DataLoader<SessionByIdLoader>,
}

impl GlobalState {
    pub fn new(config: AppConfig, db: Arc<sqlx::PgPool>, ctx: Context) -> Self {
        Self {
            config,
            ctx,
            user_by_username_loader: UserByUsernameLoader::new(db.clone()),
            user_by_id_loader: UserByIdLoader::new(db.clone()),
            session_by_id_loader: SessionByIdLoader::new(db.clone()),
            db,
        }
    }
}
