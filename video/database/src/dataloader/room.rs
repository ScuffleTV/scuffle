use std::{collections::HashMap, sync::Arc};

use async_graphql::dataloader::Loader;
use async_trait::async_trait;
use ulid::Ulid;
use uuid::Uuid;

use crate::room::Room;

pub struct RoomByNameLoader {
    db: Arc<sqlx::PgPool>,
}

#[async_trait]
impl Loader<Ulid> for RoomByNameLoader {
    type Value = Room;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[Ulid]) -> Result<HashMap<Ulid, Self::Value>, Self::Error> {
        let query: Vec<Self::Value> = sqlx::query_as(
            r#"
            SELECT * FROM rooms WHERE id = ANY($1::uuid[])
        "#,
        )
        .bind(keys.iter().map(|id| Uuid::from(*id)).collect::<Vec<_>>())
        .fetch_all(self.db.as_ref())
        .await
        .map_err(Arc::new)?;

        let mut map = HashMap::new();
        for room in query {
            map.insert(room.id.into(), room);
        }

        Ok(map)
    }
}
