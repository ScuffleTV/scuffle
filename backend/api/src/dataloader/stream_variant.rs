use crate::database::stream_variant;
use async_graphql::{
    async_trait::async_trait,
    dataloader::{DataLoader, Loader},
};
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

pub struct StreamVariantsByStreamIdLoader {
    db: Arc<sqlx::PgPool>,
}

impl StreamVariantsByStreamIdLoader {
    pub fn new(db: Arc<sqlx::PgPool>) -> DataLoader<Self> {
        DataLoader::new(Self { db }, tokio::spawn)
    }
}

#[async_trait]
impl Loader<Uuid> for StreamVariantsByStreamIdLoader {
    type Value = Vec<stream_variant::Model>;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let results = sqlx::query_as!(
            stream_variant::Model,
            "SELECT * FROM stream_variants WHERE stream_id = ANY($1)",
            &keys
        )
        .fetch_all(&*self.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch stream variants: {}", e);
            Arc::new(e)
        })?;

        let mut map = HashMap::new();

        for result in results {
            map.entry(result.stream_id)
                .or_insert_with(Vec::new)
                .push(result);
        }

        Ok(map)
    }
}
