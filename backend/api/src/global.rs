use crate::config::AppConfig;

pub struct GlobalState {
    pub config: AppConfig,
    pub db: sqlx::PgPool,
}
