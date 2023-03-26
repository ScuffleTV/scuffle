use std::sync::Arc;

use async_graphql::dataloader::DataLoader;
use common::context::Context;

use crate::config::AppConfig;
use crate::dataloader::stream::StreamByIdLoader;
use crate::dataloader::stream_variant::StreamVariantsByStreamIdLoader;
use crate::dataloader::user_permissions::UserPermissionsByIdLoader;
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
    pub user_permisions_by_id_loader: DataLoader<UserPermissionsByIdLoader>,
    pub stream_by_id_loader: DataLoader<StreamByIdLoader>,
    pub stream_variants_by_stream_id_loader: DataLoader<StreamVariantsByStreamIdLoader>,
    pub rmq: common::rmq::ConnectionPool,
}

impl GlobalState {
    pub fn new(
        config: AppConfig,
        db: Arc<sqlx::PgPool>,
        rmq: common::rmq::ConnectionPool,
        ctx: Context,
    ) -> Self {
        Self {
            config,
            ctx,
            user_by_username_loader: UserByUsernameLoader::new(db.clone()),
            user_by_id_loader: UserByIdLoader::new(db.clone()),
            session_by_id_loader: SessionByIdLoader::new(db.clone()),
            user_permisions_by_id_loader: UserPermissionsByIdLoader::new(db.clone()),
            stream_by_id_loader: StreamByIdLoader::new(db.clone()),
            stream_variants_by_stream_id_loader: StreamVariantsByStreamIdLoader::new(db.clone()),
            db,
            rmq,
        }
    }
}
