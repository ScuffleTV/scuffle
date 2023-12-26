use std::sync::Arc;

use common::dataloader::{DataLoader, Loader, LoaderOutput};
use itertools::Itertools;
use ulid::Ulid;
use video_common::database::{Recording, Rendition};

pub struct RecordingStateLoader {
	db: Arc<sqlx::PgPool>,
}

impl RecordingStateLoader {
	pub fn new(db: Arc<sqlx::PgPool>) -> DataLoader<Self> {
		DataLoader::new(Self { db })
	}
}

#[derive(Debug, Clone, Default)]
pub struct RecordingState(Vec<RecordingRenditionState>);

impl RecordingState {
	pub fn recording_to_proto(&self, recording: Recording) -> pb::scuffle::video::v1::types::Recording {
		let (size_bytes, start_time, end_time, renditions) =
			self.0
				.iter()
				.fold((0, 0.0, 0.0, Vec::new()), |(size_bytes, min, max, mut renditions), state| {
					renditions.push(state.rendition);
					(
						size_bytes + state.size_bytes,
						f32::min(min, state.start_time),
						f32::max(max, state.end_time),
						renditions,
					)
				});

		recording.into_proto(renditions, size_bytes, end_time - start_time)
	}
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RecordingRenditionState {
	pub organization_id: common::database::Ulid,
	pub recording_id: common::database::Ulid,
	pub rendition: Rendition,
	pub size_bytes: i64,
	pub end_time: f32,
	pub start_time: f32,
}

impl Loader for RecordingStateLoader {
	type Error = ();
	type Key = (Ulid, Ulid);
	type Value = RecordingState;

	async fn load(&self, key: &[Self::Key]) -> LoaderOutput<Self> {
		let ids = key.iter().copied().map(|(organization_id, recording_id)| {
			(common::database::Ulid(organization_id), common::database::Ulid(recording_id))
		});

		let mut qb = sqlx::query_builder::QueryBuilder::default();

		qb.push("SELECT organization_id, recording_id, rendition, COUNT(size_bytes) AS size_bytes, MAX(end_time) AS end_time, MAX(start_time) AS start_time FROM recording_rendition_segments WHERE (organization_id, recording_id) IN ");

		qb.push_tuples(ids, |mut qb, (organization_id, recording_id)| {
			qb.push_bind(organization_id).push_bind(recording_id);
		});

		qb.push(" GROUP BY organization_id, recording_id, rendition ORDER BY organization_id, recording_id");

		let results: Vec<RecordingRenditionState> =
			qb.build_query_as().fetch_all(self.db.as_ref()).await.map_err(|err| {
				tracing::error!(error = %err, "failed to load access tokens");
			})?;

		Ok(results
			.into_iter()
			.group_by(|v| (v.organization_id.0, v.recording_id.0))
			.into_iter()
			.map(|(k, v)| (k, RecordingState(v.collect())))
			.collect())
	}
}
