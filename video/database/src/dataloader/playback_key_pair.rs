use std::{collections::HashMap, sync::Arc};

use async_graphql::dataloader::Loader;
use async_trait::async_trait;
use ulid::Ulid;
use uuid::Uuid;

use crate::playback_key_pair::PlaybackKeyPair;

pub struct PlaybackKeyPairByNameLoader {
    db: Arc<sqlx::PgPool>,
}

#[async_trait]
impl Loader<Ulid> for PlaybackKeyPairByNameLoader {
    type Value = PlaybackKeyPair;
    type Error = Arc<sqlx::Error>;

    async fn load(
        &self,
        keys: &[Ulid],
    ) -> Result<HashMap<Ulid, Self::Value>, Self::Error> {
        let query: Vec<Self::Value> = sqlx::query_as("SELECT * FROM playback_key_pairs WHERE id = ANY($1::uuid[])")
        .bind(keys.iter().map(|id| Uuid::from(*id)).collect::<Vec<_>>())
        .fetch_all(self.db.as_ref())
        .await
        .map_err(Arc::new)?;

        let mut map = HashMap::new();
        for playback_key_pair in query {
            map.insert(
                playback_key_pair.id.into(),
                playback_key_pair,
            );
        }

        Ok(map)
    }
}
