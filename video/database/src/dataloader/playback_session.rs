use std::{collections::HashMap, sync::Arc};

use async_graphql::dataloader::Loader;
use async_trait::async_trait;
use uuid::Uuid;

use crate::playback_session::PlaybackSession;

pub struct PlaybackSessionByIdLoader {
    db: Arc<sqlx::PgPool>,
}

#[async_trait]
impl Loader<Uuid> for PlaybackSessionByIdLoader {
    type Value = PlaybackSession;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let query: Vec<Self::Value> = sqlx::query_as(
            r#"
            SELECT * FROM playback_sessions WHERE id = ANY($1)
        "#,
        )
        .bind(keys)
        .fetch_all(self.db.as_ref())
        .await
        .map_err(Arc::new)?;

        let mut map = HashMap::new();
        for playback_session in query {
            map.insert(playback_session.id, playback_session);
        }

        Ok(map)
    }
}
