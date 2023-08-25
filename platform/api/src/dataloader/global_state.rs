use std::{collections::HashMap, sync::Arc};

use async_graphql::{
    async_trait::async_trait,
    dataloader::{DataLoader, Loader},
};

use crate::database::global_state;

pub struct GlobalStateLoader {
    db: Arc<sqlx::PgPool>,
}

impl GlobalStateLoader {
    pub fn new(db: Arc<sqlx::PgPool>) -> DataLoader<Self> {
        DataLoader::new(Self { db }, tokio::spawn)
    }
}

#[async_trait]
impl Loader<()> for GlobalStateLoader {
    type Value = global_state::Model;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, _keys: &[()]) -> Result<HashMap<(), Self::Value>, Self::Error> {
        let state = sqlx::query_as("SELECT * FROM global_state")
            .fetch_one(&*self.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch roles: {}", e);
                Arc::new(e)
            })?;
        let mut map = HashMap::new();
        map.insert((), state);
        Ok(map)
    }
}
