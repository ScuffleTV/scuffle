use std::{collections::HashMap, sync::Arc};

use async_graphql::{
    async_trait::async_trait,
    dataloader::{DataLoader, Loader},
};
use uuid::Uuid;

use crate::database::role;

pub struct RoleByIdLoader {
    db: Arc<sqlx::PgPool>,
}

impl RoleByIdLoader {
    pub fn new(db: Arc<sqlx::PgPool>) -> DataLoader<Self> {
        DataLoader::new(Self { db }, tokio::spawn)
    }
}

#[async_trait]
impl Loader<Uuid> for RoleByIdLoader {
    type Value = role::Model;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let results: Vec<role::Model> = sqlx::query_as("SELECT * FROM roles WHERE id = ANY($1)")
            .bind(keys)
            .fetch_all(&*self.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch roles: {}", e);
                Arc::new(e)
            })?;

        let mut map = HashMap::new();

        for result in results {
            map.insert(result.id, result);
        }

        Ok(map)
    }
}
