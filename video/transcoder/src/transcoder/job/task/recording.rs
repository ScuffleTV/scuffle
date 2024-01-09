use std::sync::Arc;

use anyhow::Context;
use bytes::Bytes;
use futures_util::TryStreamExt;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tokio_util::compat::FuturesAsyncReadCompatExt;
use ulid::Ulid;
use video_common::database::Rendition;

use super::retry_task;
use crate::global::TranscoderGlobal;

pub enum RecordingTask {
	Segment {
		segment_id: Ulid,
		segment_idx: u32,
		duration: f64,
		start_time: f64,
		parts: Vec<Bytes>,
	},
	Init {
		data: Bytes,
	},
}

#[inline(always)]
fn normalize_float(f: f64) -> f64 {
	(f * 1000.0).round() / 1000.0
}

pub async fn recording_task(
	global: Arc<impl TranscoderGlobal>,
	organization_id: Ulid,
	recording_id: Ulid,
	rendition: Rendition,
	bucket: s3::Bucket,
	mut rx: mpsc::Receiver<RecordingTask>,
) -> anyhow::Result<()> {
	while let Some(task) = rx.recv().await {
		retry_task(
			|| async {
				match &task {
					RecordingTask::Segment {
						segment_id,
						segment_idx,
						duration,
						start_time,
						parts,
					} => {
						let size = parts.iter().map(|p| p.len()).sum::<usize>();

						let mut stream = futures_util::stream::iter(parts.clone())
							.map(std::io::Result::Ok)
							.into_async_read()
							.compat();

						let segment = video_common::keys::s3_segment(
							organization_id,
							recording_id,
							rendition,
							*segment_idx,
							*segment_id,
						);

						bucket
							.put_object_stream_with_content_type(&mut stream, &segment, "video/mp4")
							.await
							.context("upload segment")?;

						if sqlx::query(
							r#"
                        INSERT INTO recording_rendition_segments (
                            organization_id,
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
                            $7,
                            $8
                        )"#,
						)
						.bind(common::database::Ulid(organization_id))
						.bind(common::database::Ulid(recording_id))
						.bind(rendition)
						.bind(*segment_idx as i32)
						.bind(common::database::Ulid(*segment_id))
						.bind(normalize_float(*start_time))
						.bind(normalize_float(start_time + duration))
						.bind(size as i64)
						.execute(global.db().as_ref())
						.await
						.context("insert segment")?
						.rows_affected() != 1
						{
							anyhow::bail!("no rows affected");
						}
					}
					RecordingTask::Init { data } => {
						bucket
							.put_object_with_content_type(
								video_common::keys::s3_init(organization_id, recording_id, rendition),
								&data,
								"video/mp4",
							)
							.await
							.context("upload init")?;
					}
				}

				Ok(())
			},
			5,
		)
		.await
		.context("s3_recording_task")?;
	}

	Ok(())
}

pub struct RecordingThumbnailTask {
	pub idx: u32,
	pub id: Ulid,
	pub start_time: f64,
	pub data: Bytes,
}

pub async fn recording_thumbnail_task(
	global: Arc<impl TranscoderGlobal>,
	organization_id: Ulid,
	recording_id: Ulid,
	bucket: s3::Bucket,
	mut rx: mpsc::Receiver<RecordingThumbnailTask>,
) -> anyhow::Result<()> {
	while let Some(task) = rx.recv().await {
		retry_task(
			|| async {
				let mut cursor = std::io::Cursor::new(&task.data);
				let size = task.data.len();

				bucket
					.put_object_stream_with_content_type(
						&mut cursor,
						&video_common::keys::s3_thumbnail(organization_id, recording_id, task.idx, task.id),
						"image/jpg",
					)
					.await
					.context("upload thumbnail")?;

				if sqlx::query(
					r#"
                INSERT INTO recording_thumbnails (
                    organization_id,
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
				.bind(common::database::Ulid(organization_id))
				.bind(common::database::Ulid(recording_id))
				.bind(task.idx as i32)
				.bind(common::database::Ulid(task.id))
				.bind(normalize_float(task.start_time))
				.bind(size as i64)
				.execute(global.db().as_ref())
				.await
				.context("insert thumbnail")?
				.rows_affected() != 1
				{
					anyhow::bail!("no rows affected");
				}

				Ok(())
			},
			5,
		)
		.await
		.context("s3_thumbnail_task")?;
	}

	Ok(())
}
