use std::sync::Arc;

use async_nats::jetstream::object_store::ObjectMetadata;
use bytes::Bytes;
use futures_util::{FutureExt, StreamExt, TryStreamExt};
use tokio_util::compat::FuturesAsyncReadCompatExt;
use ulid::Ulid;
use uuid::Uuid;
use video_common::database::Rendition;

pub type TaskFuture = video_common::tasker::TaskFuture<(), TaskError>;
pub type Task<G> = video_common::tasker::Task<Arc<G>, TaskDomain, (), TaskError>;
pub type MultiTasker<G> = video_common::tasker::MultiTasker<Arc<G>, TaskDomain, (), TaskError>;

use crate::global::TranscoderGlobal;

use super::SegmentUpload;

#[derive(Debug, thiserror::Error)]
pub enum TaskError {
    #[error("failed to upload metadata: {0}")]
    NatsKvPut(#[from] async_nats::jetstream::kv::PutError),
    #[error("failed to upload media: {0}")]
    NatsObjPut(#[from] async_nats::jetstream::object_store::PutError),
    #[error("failed to delete metadata: {0}")]
    NatsKvUpdate(#[from] async_nats::jetstream::kv::UpdateError),
    #[error("failed to delete media: {0}")]
    NatsObjDelete(#[from] async_nats::jetstream::object_store::DeleteError),
    #[error("failed to upload recording: {0}")]
    S3(#[from] s3::error::S3Error),
    #[error("custom task failed: {0}")]
    Custom(#[from] anyhow::Error),
}

pub fn upload_metadata_generator<G: TranscoderGlobal>(
    key: String,
    data: Bytes,
) -> impl Fn(Arc<G>) -> TaskFuture + Send + Sync + 'static {
    move |global: Arc<G>| {
        let global = global.clone();
        let data = data.clone();
        let key = key.clone();
        async move {
            global.metadata_store().put(key, data).await?;
            Ok(())
        }
        .boxed()
    }
}

pub fn upload_media_generator<G: TranscoderGlobal>(
    key: String,
    data: Bytes,
) -> impl Fn(Arc<G>) -> TaskFuture + Send + Sync + 'static {
    move |global: Arc<G>| {
        let global = global.clone();
        let data = data.clone();
        let key = ObjectMetadata::from(key.as_str());
        async move {
            let mut cursor = std::io::Cursor::new(data);
            global.media_store().put(key, &mut cursor).await?;
            Ok(())
        }
        .boxed()
    }
}

pub fn delete_media_generator<G: TranscoderGlobal>(
    key: String,
) -> impl Fn(Arc<G>) -> TaskFuture + Send + Sync + 'static {
    move |global: Arc<G>| {
        let global = global.clone();
        let key = key.clone();
        async move {
            global.media_store().delete(key).await?;
            Ok(())
        }
        .boxed()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum TaskDomain {
    Generic,
    Thumbnail,
    Normal(Rendition),
    Recording(Rendition),
}

#[derive(Clone)]
pub struct RecordingState {
    pub recording_id: Ulid,
    pub organization_id: Ulid,
    pub bucket: Arc<s3::Bucket>,
}

#[inline(always)]
fn normalize_float(f: f64) -> f64 {
    (f * 1000.0).round() / 1000.0
}

pub fn upload_segment_generator<G: TranscoderGlobal>(
    state: RecordingState,
    upload: SegmentUpload,
) -> impl Fn(Arc<G>) -> TaskFuture + Send + Sync + 'static {
    move |global| {
        let state = state.clone();
        let upload = upload.clone();
        Box::pin(async move {
            let size = upload.parts.iter().map(|p| p.len()).sum::<usize>();

            let mut stream = futures_util::stream::iter(upload.parts)
                .map(std::io::Result::Ok)
                .into_async_read()
                .compat();

            state
                .bucket
                .put_object_stream_with_content_type(
                    &mut stream,
                    &video_common::keys::s3_segment(
                        state.organization_id,
                        state.recording_id,
                        upload.rendition,
                        upload.segment_idx,
                        upload.segment_id,
                    ),
                    "video/mp4",
                )
                .await?;

            if sqlx::query(
                r#"
            INSERT INTO recording_rendition_segments (
                recording_id,
                rendition,
                idx,
                id,
                start_time,
                end_time,
                size_bytes
            ) VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                $7
            )"#,
            )
            .bind(Uuid::from(state.recording_id))
            .bind(upload.rendition)
            .bind(upload.segment_idx as i32)
            .bind(Uuid::from(upload.segment_id))
            .bind(normalize_float(upload.start_time))
            .bind(normalize_float(upload.start_time + upload.duration))
            .bind(size as i64)
            .execute(global.db().as_ref())
            .await
            .map_err(|e| TaskError::Custom(e.into()))?
            .rows_affected()
                != 1
            {
                return Err(TaskError::Custom(anyhow::anyhow!(
                    "Failed to update recording rendition"
                )));
            }

            Ok(())
        })
    }
}

#[derive(Clone)]
pub struct ThumbnailUpload {
    pub idx: u32,
    pub id: Ulid,
    pub start_time: f64,
    pub data: Bytes,
}

pub fn upload_thumbnail_generator<G: TranscoderGlobal>(
    state: RecordingState,
    upload: ThumbnailUpload,
) -> impl Fn(Arc<G>) -> TaskFuture + Send + Sync + 'static {
    move |global| {
        let state = state.clone();
        let partial = upload.clone();
        Box::pin(async move {
            let mut cursor = std::io::Cursor::new(&partial.data);
            let size = partial.data.len();

            state
                .bucket
                .put_object_stream_with_content_type(
                    &mut cursor,
                    &video_common::keys::s3_thumbnail(
                        state.organization_id,
                        state.recording_id,
                        partial.idx,
                        partial.id,
                    ),
                    "image/jpg",
                )
                .await?;

            if sqlx::query(
                r#"
            INSERT INTO recording_thumbnails (
                recording_id,
                idx,
                id,
                start_time,
                size_bytes
            ) VALUES (
                $1,
                $2,
                $3,
                $4,
                $5
            )"#,
            )
            .bind(Uuid::from(state.recording_id))
            .bind(partial.idx as i32)
            .bind(Uuid::from(partial.id))
            .bind(normalize_float(partial.start_time))
            .bind(size as i64)
            .execute(global.db().as_ref())
            .await
            .map_err(|e| TaskError::Custom(e.into()))?
            .rows_affected()
                != 1
            {
                return Err(TaskError::Custom(anyhow::anyhow!(
                    "Failed to update recording rendition"
                )));
            }

            Ok(())
        })
    }
}
