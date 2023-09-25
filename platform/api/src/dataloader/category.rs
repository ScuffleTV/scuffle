use std::{collections::HashMap, sync::Arc};

use common::dataloader::{DataLoader, Loader, LoaderOutput};
use ulid::Ulid;
use uuid::Uuid;

use crate::database::{Category, SearchResult};

pub struct CategoryByIdLoader {
    db: Arc<sqlx::PgPool>,
}

impl CategoryByIdLoader {
    pub fn new(db: Arc<sqlx::PgPool>) -> DataLoader<Self> {
        DataLoader::new(Self { db })
    }
}

#[async_trait::async_trait]
impl Loader for CategoryByIdLoader {
    type Key = Ulid;
    type Value = Category;
    type Error = ();

    async fn load(&self, keys: &[Self::Key]) -> LoaderOutput<Self> {
        let results: Vec<Self::Value> =
            sqlx::query_as("SELECT * FROM categories WHERE id = ANY($1)")
                .bind(keys.iter().copied().map(Uuid::from).collect::<Vec<_>>())
                .fetch_all(self.db.as_ref())
                .await
                .map_err(|e| {
                    tracing::error!(err = %e, "failed to fetch categories by id");
                })?;

        let mut map = HashMap::new();

        for result in results {
            map.insert(result.id.0, result);
        }

        Ok(map)
    }
}

pub struct CategorySearchLoader {
    db: Arc<sqlx::PgPool>,
}

impl CategorySearchLoader {
    pub fn new(db: Arc<sqlx::PgPool>) -> DataLoader<Self> {
        DataLoader::new(Self { db })
    }
}

#[async_trait::async_trait]
impl Loader for CategorySearchLoader {
    type Key = String;
    type Value = Vec<SearchResult<Category>>;
    type Error = ();

    async fn load(&self, keys: &[Self::Key]) -> LoaderOutput<Self> {
        let mut map = HashMap::new();

        for key in keys {
            let results: Self::Value =
                sqlx::query_as("SELECT categories.*, similarity(name, $1) FROM categories WHERE name % $1 ORDER BY similarity DESC LIMIT 5")
                    .bind(key)
                    .fetch_all(self.db.as_ref())
                    .await
                    .map_err(|e| {
                        tracing::error!(err = %e, "failed to search categories by search");
                    })?;

            map.insert(key.clone(), results);
        }

        Ok(map)
    }
}
