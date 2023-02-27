use crate::config::AppConfig;
use std::sync::Arc;

use async_graphql::dataloader::DataLoader;
use common::context::Context;

use crate::dataloader::{
    session::SessionByIdLoader, user::UserByIdLoader, user::UserByUsernameLoader,
};
use fred::clients::SubscriberClient;
use fred::pool::RedisPool;

pub mod turnstile;

pub struct GlobalState {
    pub config: AppConfig,
    pub db: Arc<sqlx::PgPool>,
    pub ctx: Context,
    pub user_by_username_loader: DataLoader<UserByUsernameLoader>,
    pub user_by_id_loader: DataLoader<UserByIdLoader>,
    pub session_by_id_loader: DataLoader<SessionByIdLoader>,
    pub redis_pool: RedisPool,
    pub redis_sub_client: SubscriberClient,
}

impl GlobalState {
    pub fn new(config: AppConfig, db: Arc<sqlx::PgPool>, ctx: Context, redis_pool: RedisPool, redis_sub_client: SubscriberClient) -> Self {
        Self {
            config,
            ctx,
            user_by_username_loader: UserByUsernameLoader::new(db.clone()),
            user_by_id_loader: UserByIdLoader::new(db.clone()),
            session_by_id_loader: SessionByIdLoader::new(db.clone()),
            db,
            redis_pool,
            redis_sub_client
        }
    }
}
