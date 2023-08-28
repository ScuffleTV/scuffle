use crate::database::stream;
use async_graphql::{
    async_trait::async_trait,
    dataloader::{DataLoader, Loader},
};
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

pub struct StreamByIdLoader {
    db: Arc<sqlx::PgPool>,
}

impl StreamByIdLoader {
    pub fn new(db: Arc<sqlx::PgPool>) -> DataLoader<Self> {
        DataLoader::new(Self { db }, tokio::spawn)
    }
}

#[async_trait]
impl Loader<Uuid> for StreamByIdLoader {
    type Value = stream::Model;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, _keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        // let results: Vec<stream::Model> = sqlx::query_as(
        //     "SELECT * FROM streams WHERE id = ANY($1)",
        // )
        // .bind(keys)
        // .fetch_all(&*self.db)
        // .await
        // .map_err(|e| {
        //     tracing::error!("Failed to fetch streams: {}", e);
        //     Arc::new(e)
        // })?;

        // let mut map = HashMap::new();

        // for result in results {
        //     map.insert(result.id, result);
        // }

        // Ok(map)
        todo!()
    }
}

/// Fetches the active stream for each user in the given list.
pub struct ActiveStreamsByUserIdLoader {
    db: Arc<sqlx::PgPool>,
}

impl ActiveStreamsByUserIdLoader {
    pub fn new(db: Arc<sqlx::PgPool>) -> DataLoader<Self> {
        DataLoader::new(Self { db }, tokio::spawn)
    }
}

#[async_trait]
impl Loader<Uuid> for ActiveStreamsByUserIdLoader {
    type Value = stream::Model;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, _keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        // let results: Vec<stream::Model> = sqlx::query_as(
        //     "SELECT * FROM streams WHERE channel_id = ANY($1) AND deleted = false AND ready_state = 1 ORDER BY created_at DESC",
        // )
        // .bind(keys)
        // .fetch_all(&*self.db)
        // .await
        // .map_err(|e| {
        //     tracing::error!("Failed to fetch streams: {}", e);
        //     Arc::new(e)
        // })?;

        // let mut map = HashMap::new();

        // for result in results {
        //     map.insert(result.channel_id, result);
        // }

        todo!()

        // Ok(map)
    }
}
