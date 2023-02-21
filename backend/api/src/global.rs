use common::context::Context;

use crate::config::AppConfig;

pub struct GlobalState {
    pub config: AppConfig,
    pub db: sqlx::PgPool,
    pub ctx: Context,
}
