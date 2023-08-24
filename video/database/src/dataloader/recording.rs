use std::{collections::HashMap, sync::Arc};

use async_graphql::dataloader::Loader;
use async_trait::async_trait;
use uuid::Uuid;

use crate::recording::Recording;

pub struct RecordingByIdLoader {
    db: Arc<sqlx::PgPool>,
}

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct RecordingInfo {
    pub recording_id: Uuid,
    pub total_size: i64,
    pub recording_duration: f64,
}

#[async_trait]
impl Loader<Uuid> for RecordingByIdLoader {
    type Value = (Recording, RecordingInfo);
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let recording_query: Vec<Recording> = sqlx::query_as(
            r#"
            SELECT * FROM recordings WHERE id = ANY($1)
        "#,
        )
        .bind(keys)
        .fetch_all(self.db.as_ref())
        .await
        .map_err(Arc::new)?;

        let mut map = HashMap::<Uuid, Self::Value>::new();
        for recording in recording_query {
            map.insert(recording.id, (recording, Default::default()));
        }

        let recording_segments_query: Vec<RecordingInfo> = if map.is_empty() {
            Vec::new()
        } else {
            sqlx::query_as(
                r#"
                WITH DistinctDurationsAndSizes AS (
                    SELECT 
                        recording_id,
                        SUM(segment_end - segment_start) AS rendition_duration,
                        SUM(size_bytes) AS rendition_size
                    FROM
                        recording_segments
                    WHERE 
                        recording_id = ANY($1)
                    GROUP BY 
                        recording_id, segment_number
                )
                SELECT
                    recording_id,
                    SUM(rendition_size) AS total_size,
                    EXTRACT(EPOCH FROM MAX(rendition_duration)) AS recording_duration
                FROM 
                    DistinctDurationsAndSizes
                GROUP BY
                    recording_id;
            "#,
            )
            .bind(map.keys().cloned().collect::<Vec<_>>())
            .fetch_all(self.db.as_ref())
            .await
            .map_err(Arc::new)?
        };

        for recording in recording_segments_query {
            let Some(recording_info) = map.get_mut(&recording.recording_id) else {
                continue;
            };

            let _ = std::mem::replace(&mut recording_info.1, recording);
        }

        Ok(map)
    }
}
