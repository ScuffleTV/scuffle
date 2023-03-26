use crate::database::session;
use async_graphql::{
    async_trait::async_trait,
    dataloader::{DataLoader, Loader},
};
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

pub struct SessionByIdLoader {
    db: Arc<sqlx::PgPool>,
}

impl SessionByIdLoader {
    pub fn new(db: Arc<sqlx::PgPool>) -> DataLoader<Self> {
        DataLoader::new(Self { db }, tokio::spawn)
    }
}

#[async_trait]
impl Loader<Uuid> for SessionByIdLoader {
    type Value = session::Model;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let results = sqlx::query_as!(
            session::Model,
            "SELECT * FROM sessions WHERE id = ANY($1)",
            &keys
        )
        .fetch_all(&*self.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch sessions: {}", e);
            Arc::new(e)
        })?;

        let mut map = HashMap::new();

        for result in results {
            map.insert(result.id, result);
        }

        Ok(map)
    }
}
