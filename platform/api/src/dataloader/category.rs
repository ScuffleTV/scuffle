use std::{collections::HashMap, sync::Arc};

use async_graphql::{
    async_trait::async_trait,
    dataloader::{DataLoader, Loader},
};
use uuid::Uuid;

use crate::database::category;

pub struct CategoryByIdLoader {
    db: Arc<sqlx::PgPool>,
}

impl CategoryByIdLoader {
    pub fn new(db: Arc<sqlx::PgPool>) -> DataLoader<Self> {
        DataLoader::new(Self { db }, tokio::spawn)
    }
}

#[async_trait]
impl Loader<Uuid> for CategoryByIdLoader {
    type Value = category::Model;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let results: Vec<category::Model> =
            sqlx::query_as("SELECT * FROM categories WHERE id = ANY($1)")
                .bind(keys)
                .fetch_all(&*self.db)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to fetch categories: {}", e);
                    Arc::new(e)
                })?;

        let mut map = HashMap::new();

        for result in results {
            map.insert(result.id, result);
        }

        Ok(map)
    }
}

pub struct CategorySearchLoader {
    db: Arc<sqlx::PgPool>,
}

impl CategorySearchLoader {
    pub fn new(db: Arc<sqlx::PgPool>) -> DataLoader<Self> {
        DataLoader::new(Self { db }, tokio::spawn)
    }
}

#[async_trait]
impl Loader<String> for CategorySearchLoader {
    type Value = Vec<category::SearchResult>;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[String]) -> Result<HashMap<String, Self::Value>, Self::Error> {
        let mut map = HashMap::new();

        for key in keys {
            let results: Vec<category::SearchResult> =
                sqlx::query_as("SELECT categories.*, similarity(name, $1) FROM categories WHERE name % $1 ORDER BY similarity DESC LIMIT 5")
                    .bind(key)
                    .fetch_all(&*self.db)
                    .await
                    .map_err(|e| {
                        tracing::error!("failed to search categories: {}", e);
                        Arc::new(e)
                    })?;

            map.insert(key.clone(), results);
        }

        Ok(map)
    }
}
