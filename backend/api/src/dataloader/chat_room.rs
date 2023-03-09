use async_graphql::{
    async_trait::async_trait,
    dataloader::{DataLoader, Loader},
};
use common::types::chat_room;
use std::{collections::HashMap, sync::Arc};

pub struct ChatRoomByIdLoader {
    db: Arc<sqlx::PgPool>,
}

impl ChatRoomByIdLoader {
    pub fn new(db: Arc<sqlx::PgPool>) -> DataLoader<Self> {
        DataLoader::new(Self { db }, tokio::spawn)
    }
}

#[async_trait]
impl Loader<i64> for ChatRoomByIdLoader {
    type Value = chat_room::Model;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[i64]) -> Result<HashMap<i64, Self::Value>, Self::Error> {
        let results = sqlx::query_as!(
            chat_room::Model,
            "SELECT * FROM chat_rooms WHERE id = ANY($1)",
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
