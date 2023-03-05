use async_graphql::{
    async_trait::async_trait,
    dataloader::{DataLoader, Loader},
};
use common::types::session;
use std::{collections::HashMap, sync::Arc};

pub struct SessionByIdLoader {
    db: Arc<sqlx::PgPool>,
}

impl SessionByIdLoader {
    pub fn new(db: Arc<sqlx::PgPool>) -> DataLoader<Self> {
        DataLoader::new(Self { db }, tokio::spawn)
    }
}

#[async_trait]
impl Loader<i64> for SessionByIdLoader {
    type Value = session::Model;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[i64]) -> Result<HashMap<i64, Self::Value>, Self::Error> {
        let results = sqlx::query_as!(
            session::Model,
            "SELECT * FROM sessions WHERE id = ANY($1)",
            &keys
        )
        .fetch_all(&*self.db)
        .await?;

        let mut map = HashMap::new();

        for result in results {
            map.insert(result.id, result);
        }

        Ok(map)
    }
}
