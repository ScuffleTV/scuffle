use common::dataloader::{DataLoader, Loader, LoaderOutput};
use std::{collections::HashMap, sync::Arc};
use ulid::Ulid;
use uuid::Uuid;

use crate::database::Session;

pub struct SessionByIdLoader {
    db: Arc<sqlx::PgPool>,
}

impl SessionByIdLoader {
    pub fn new(db: Arc<sqlx::PgPool>) -> DataLoader<Self> {
        DataLoader::new(Self { db })
    }
}

#[async_trait::async_trait]
impl Loader for SessionByIdLoader {
    type Key = Ulid;
    type Value = Session;
    type Error = ();

    async fn load(&self, keys: &[Self::Key]) -> LoaderOutput<Self> {
        let results: Vec<Self::Value> =
            sqlx::query_as("SELECT * FROM user_sessions WHERE id = ANY($1)")
                .bind(keys.iter().copied().map(Uuid::from).collect::<Vec<_>>())
                .fetch_all(self.db.as_ref())
                .await
                .map_err(|e| {
                    tracing::error!(err = %e, "failed to fetch sessions");
                })?;

        let mut map = HashMap::new();

        for result in results {
            map.insert(result.id.0, result);
        }

        Ok(map)
    }
}
