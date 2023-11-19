use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use anyhow::Result;
use bytes::Bytes;
use pb::ext::UlidExt;
use pb::scuffle::video::internal::live_rendition_manifest::recording_data::RecordingThumbnail;
use pb::scuffle::video::v1::types::{AudioConfig, RecordingConfig, VideoConfig};
use prost::Message;
use s3::Region;
use ulid::Ulid;
use uuid::Uuid;
use video_common::database::{Rendition, S3Bucket};

use super::{upload_segment_generator, upload_thumbnail_generator, RecordingState, Task, TaskDomain, ThumbnailUpload};
use crate::global::TranscoderGlobal;

#[derive(Clone)]
pub struct SegmentUpload {
	pub segment_id: Ulid,
	pub rendition: Rendition,
	pub segment_idx: u32,
	pub duration: f64,
	pub start_time: f64,
	pub parts: Vec<Bytes>,
}

pub struct Recording {
	id: Ulid,
	organization_id: Ulid,
	allow_dvr: bool,
	bucket: Arc<s3::Bucket>,
	partial_uploads: HashMap<Rendition, SegmentUpload>,
	renditions: HashSet<Rendition>,
	recent_thumbnails: VecDeque<ThumbnailUpload>,
}

impl Recording {
	#[allow(clippy::too_many_arguments)]
	pub async fn new(
		tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
		id: Ulid,
		organization_id: Ulid,
		room_id: Ulid,
		public: bool,
		audio_outputs: &[AudioConfig],
		video_outputs: &[VideoConfig],
		s3_bucket: &S3Bucket,
		recording_config: &RecordingConfig,
	) -> Result<Self> {
		let bucket = Arc::new(
			s3::Bucket::new(
				&s3_bucket.name,
				{
					let region = s3_bucket
						.endpoint
						.as_ref()
						.or(Some(&s3_bucket.region))
						.and_then(|s| s.parse().ok())
						.ok_or_else(|| anyhow::anyhow!("Invalid S3 region: {:?}", s3_bucket.region))?;
					match region {
						Region::Custom { endpoint, .. } => s3::Region::Custom {
							region: Region::UsEast1.to_string(),
							endpoint,
						},
						_ => region,
					}
				},
				s3::creds::Credentials {
					access_key: Some(s3_bucket.access_key_id.clone()),
					secret_key: Some(s3_bucket.secret_access_key.clone()),
					security_token: None,
					session_token: None,
					expiration: None,
				},
			)?
			.with_path_style(),
		);

		let allow_dvr = audio_outputs
			.iter()
			.map(|o| o.rendition)
			.chain(video_outputs.iter().map(|o| o.rendition))
			.all(|r| recording_config.renditions.contains(&r));

		sqlx::query(
			r#"
			INSERT INTO recordings (
                id,
                organization_id,
                room_id,
                recording_config_id,
                public,
                allow_dvr,
                s3_bucket_id
            ) VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                $7
            ) ON CONFLICT DO NOTHING
			"#,
		)
		.bind(Uuid::from(id))
		.bind(Uuid::from(organization_id))
		.bind(Uuid::from(room_id))
		.bind(Uuid::from(recording_config.id.to_ulid()))
		.bind(public)
		.bind(allow_dvr)
		.bind(s3_bucket.id)
		.execute(tx.as_mut())
		.await?;

		let mut qb = sqlx::QueryBuilder::new("INSERT INTO recording_renditions (recording_id, rendition, config)");

		let video_outputs = video_outputs
			.iter()
			.map(|o| (Rendition::from(o.rendition()), o.encode_to_vec()));
		let audio_outputs = audio_outputs
			.iter()
			.map(|o| (Rendition::from(o.rendition()), o.encode_to_vec()));

		qb.push_values(video_outputs.chain(audio_outputs), |mut b, (rendition, config)| {
			b.push_bind(Uuid::from(id));
			b.push_bind(rendition);
			b.push_bind(config);
		});

		qb.push("ON CONFLICT DO NOTHING");

		qb.build().execute(tx.as_mut()).await?;

		Ok(Self {
			id,
			organization_id,
			bucket,
			allow_dvr,
			recent_thumbnails: VecDeque::new(),
			renditions: recording_config
				.renditions
				.iter()
				.map(|r| Rendition::from(pb::scuffle::video::v1::types::Rendition::try_from(*r).unwrap()))
				.collect(),
			partial_uploads: HashMap::new(),
		})
	}

	pub fn recover_recordings(&mut self, recordings: &[RecordingThumbnail]) {
		self.recent_thumbnails = recordings
			.iter()
			.map(|r| ThumbnailUpload {
				idx: r.idx,
				id: r.ulid.to_ulid(),
				data: Bytes::new(),
				start_time: r.timestamp as f64,
			})
			.collect();
	}

	pub fn id(&self) -> Ulid {
		self.id
	}

	pub fn allow_dvr(&self) -> bool {
		self.allow_dvr
	}

	pub fn renditions(&self) -> &HashSet<Rendition> {
		&self.renditions
	}

	pub fn recent_thumbnails(&self) -> impl Iterator<Item = &ThumbnailUpload> {
		self.recent_thumbnails.iter()
	}

	#[must_use]
	#[allow(clippy::too_many_arguments)]
	pub fn upload_part<G: TranscoderGlobal>(
		&mut self,
		rendition: Rendition,
		id: Ulid,
		idx: u32,
		data: Bytes,
		start_time: f64,
		duration: f64,
		finished: bool,
	) -> Option<Task<G>> {
		let partial_upload = self.partial_uploads.entry(rendition).or_insert_with(|| SegmentUpload {
			segment_id: id,
			segment_idx: idx,
			duration,
			rendition,
			start_time,
			parts: Vec::new(),
		});

		if partial_upload.segment_id != id || finished {
			let partial_upload = std::mem::replace(
				partial_upload,
				SegmentUpload {
					segment_id: id,
					segment_idx: idx,
					duration,
					rendition,
					start_time,
					parts: vec![data],
				},
			);

			let state = RecordingState {
				recording_id: self.id,
				organization_id: self.organization_id,
				bucket: self.bucket.clone(),
			};

			Some(Task::new(
				format!("segment_{segment_idx}", segment_idx = partial_upload.segment_idx),
				Arc::new(upload_segment_generator(state, partial_upload)),
				TaskDomain::Recording(rendition),
			))
		} else {
			partial_upload.parts.push(data);
			partial_upload.duration = duration;
			None
		}
	}

	pub fn upload_init<G: TranscoderGlobal>(&mut self, rendition: Rendition, data: Bytes) -> Task<G> {
		let state = RecordingState {
			recording_id: self.id,
			organization_id: self.organization_id,
			bucket: self.bucket.clone(),
		};

		Task::new(
			"init".into(),
			Arc::new(move |_| {
				let state = state.clone();
				let data = data.clone();
				Box::pin(async move {
					state
						.bucket
						.put_object_with_content_type(
							video_common::keys::s3_init(state.organization_id, state.recording_id, rendition),
							&data,
							"video/mp4",
						)
						.await?;
					Ok(())
				})
			}),
			TaskDomain::Recording(rendition),
		)
	}

	pub fn upload_thumbnail<G: TranscoderGlobal>(&mut self, id: Ulid, idx: u32, start_time: f64, data: Bytes) -> Task<G> {
		let state = RecordingState {
			recording_id: self.id,
			organization_id: self.organization_id,
			bucket: self.bucket.clone(),
		};

		self.recent_thumbnails.push_back(ThumbnailUpload {
			idx,
			id,
			data: Bytes::new(),
			start_time,
		});

		if self.recent_thumbnails.len() > 5 {
			self.recent_thumbnails.pop_front();
		}

		Task::new(
			format!("thumbnail_{idx}", idx = idx),
			Arc::new(upload_thumbnail_generator(
				state,
				ThumbnailUpload {
					idx,
					id,
					data,
					start_time,
				},
			)),
			TaskDomain::Thumbnail,
		)
	}
}
