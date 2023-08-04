use std::{sync::Arc, time::Duration};

use common::prelude::FutureTimeout;
use tokio::sync::mpsc;
use ulid::Ulid;
use uuid::Uuid;

use crate::global::GlobalState;

pub struct Update {
    pub bitrate: i32,
}

pub async fn update_db(
    global: Arc<GlobalState>,
    id: Ulid,
    organization_id: Ulid,
    room_id: Ulid,
    mut update_reciever: mpsc::Receiver<Update>,
) {
    while let Some(update) = update_reciever.recv().await {
        let mut success = false;

        for _ in 0..5 {
            match sqlx::query(
                r#"
                UPDATE rooms
                SET
                    updated_at = NOW(),
                    ingest_bitrate = $1
                WHERE
                    organization_id = $2 AND
                    id = $3 AND
                    active_ingest_connection_id = $4
                "#,
            )
            .bind(update.bitrate)
            .bind(Uuid::from(organization_id))
            .bind(Uuid::from(room_id))
            .bind(Uuid::from(id))
            .execute(global.db.as_ref())
            .timeout(Duration::from_secs(3))
            .await
            {
                Ok(Ok(r)) => {
                    if r.rows_affected() != 1 {
                        tracing::error!("failed to update api with bitrate - no rows affected");
                        return;
                    } else {
                        success = true;
                        break;
                    }
                }
                Ok(Err(e)) => {
                    tracing::error!(error = %e, "failed to update api with bitrate");
                }
                Err(_) => {
                    tracing::error!("failed to update api with bitrate timedout");
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        if !success {
            tracing::error!("failed to update api with bitrate after 5 retries - giving up");
            return;
        }
    }
}
