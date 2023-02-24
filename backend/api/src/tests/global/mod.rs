use std::sync::Arc;

use common::{
    context::{Context, Handler},
    logging,
};

use crate::{config::AppConfig, global::GlobalState};

pub mod turnstile;

pub async fn mock_global_state(config: AppConfig) -> (Arc<GlobalState>, Handler) {
    let (ctx, handler) = Context::new();

    dotenvy::dotenv().ok();

    logging::init("api=debug").expect("failed to initialize logging");

    let db = sqlx::PgPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL not set"))
        .await
        .expect("failed to connect to database");

    (Arc::new(GlobalState { config, db, ctx }), handler)
}
