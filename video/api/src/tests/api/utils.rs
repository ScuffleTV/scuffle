use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;

use common::global::GlobalDb;
use itertools::Itertools;
use ulid::Ulid;
use video_common::database::{AccessToken, Rendition};

use crate::api::playback_key_pair::utils::validate_public_key;
use crate::api::room::utils::create_stream_key;
use crate::api::utils::ApiRequest;
use crate::tests::global::GlobalState;

// Helper function for processing requests
pub async fn process_request<T, R>(global: &Arc<GlobalState>, access_token: &AccessToken, request: T) -> tonic::Result<R>
where
	tonic::Request<T>: ApiRequest<R> + Send + Sync + 'static,
	R: std::fmt::Debug + Send + Sync + 'static,
{
	tonic::Request::new(request)
		.process(global, access_token)
		.await
		.map(|r| r.into_inner())
}

// Helper function for asserting query matches
pub fn assert_query_matches(
	result: tonic::Result<sqlx::query_builder::QueryBuilder<'_, sqlx::Postgres>>,
	expected: Result<&str, &str>,
) {
	match (result, expected) {
		(Ok(result), Ok(expected)) => assert_eq!(result.into_sql(), expected),
		(Err(result), Err(expected)) => assert_eq!(result.message(), expected),
		(Err(result), Ok(expected)) => panic!("query does not match: {result:#?} != {expected:#?}"),
		(Ok(result), Err(expected)) => panic!(
			"query does not match: {result:#?} != {expected:#?}",
			result = result.into_sql()
		),
	}
}

pub async fn create_playback_session(
	global: &Arc<GlobalState>,
	organization_id: Ulid,
	inserts: impl Iterator<Item = (Option<Ulid>, Option<Ulid>, Option<String>, IpAddr)>,
) -> Vec<video_common::database::PlaybackSession> {
	let mut results = Vec::new();

	for inserts in &inserts.chunks(u16::MAX as usize / 5) {
		let mut qb = sqlx::query_builder::QueryBuilder::default();

		qb.push("INSERT INTO playback_sessions (id, organization_id, room_id, recording_id, user_id, ip_address) ");

		qb.push_values(inserts, |mut qb, values| {
			qb.push_bind(common::database::Ulid(Ulid::new()));
			qb.push_bind(common::database::Ulid(organization_id));
			qb.push_bind(values.0.map(common::database::Ulid));
			qb.push_bind(values.1.map(common::database::Ulid));
			qb.push_bind(values.2);
			qb.push_bind(values.3);
		});

		qb.push(" RETURNING *");

		results.extend(qb.build_query_as().fetch_all(global.db().as_ref()).await.unwrap());
	}

	results
}

pub async fn create_room(global: &Arc<GlobalState>, organization_id: Ulid) -> video_common::database::Room {
	sqlx::query_as("INSERT INTO rooms (id, organization_id, stream_key) VALUES ($1, $2, $3) RETURNING *")
		.bind(common::database::Ulid(Ulid::new()))
		.bind(common::database::Ulid(organization_id))
		.bind(create_stream_key())
		.fetch_one(global.db().as_ref())
		.await
		.unwrap()
}

pub async fn create_recording(
	global: &Arc<GlobalState>,
	organization_id: Ulid,
	s3_bucket_id: Ulid,
	room_id: Option<Ulid>,
	recording_config_id: Option<Ulid>,
	tags: HashMap<String, String>,
) -> video_common::database::Recording {
	sqlx::query_as("INSERT INTO recordings (id, organization_id, s3_bucket_id, room_id, recording_config_id, tags) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *")
		.bind(common::database::Ulid(Ulid::new()))
		.bind(common::database::Ulid(organization_id))
		.bind(common::database::Ulid(s3_bucket_id))
		.bind(room_id.map(common::database::Ulid))
		.bind(recording_config_id.map(common::database::Ulid))
		.bind(sqlx::types::Json(tags))
		.fetch_one(global.db().as_ref())
		.await
		.unwrap()
}

pub async fn create_recording_thumbnail(
	global: &Arc<GlobalState>,
	organization_id: Ulid,
	recording_id: Ulid,
	inserts: impl Iterator<Item = (i32, f32)>,
) -> Vec<video_common::database::RecordingThumbnail> {
	let mut results = Vec::new();

	for inserts in &inserts.chunks(u16::MAX as usize / 5) {
		let mut qb = sqlx::query_builder::QueryBuilder::default();

		qb.push("INSERT INTO recording_thumbnails (organization_id, recording_id, idx, id, start_time) ");

		qb.push_values(inserts, |mut qb, values| {
			qb.push_bind(common::database::Ulid(organization_id));
			qb.push_bind(common::database::Ulid(recording_id));
			qb.push_bind(values.0);
			qb.push_bind(common::database::Ulid(Ulid::new()));
			qb.push_bind(values.1);
		});

		qb.push(" RETURNING *");

		results.extend(qb.build_query_as().fetch_all(global.db().as_ref()).await.unwrap());
	}

	results
}

pub async fn create_recording_segment(
	global: &Arc<GlobalState>,
	organization_id: Ulid,
	recording_id: Ulid,
	inserts: impl Iterator<Item = (Rendition, i32, f32, f32)>,
) -> Vec<video_common::database::RecordingRenditionSegment> {
	let mut results = Vec::new();

	for inserts in &inserts.chunks(u16::MAX as usize / 7) {
		let mut qb = sqlx::query_builder::QueryBuilder::default();

		qb.push(
			"INSERT INTO recording_rendition_segments (organization_id, recording_id, rendition, idx, id, start_time, end_time) ",
		);

		qb.push_values(inserts, |mut qb, values| {
			qb.push_bind(common::database::Ulid(organization_id));
			qb.push_bind(common::database::Ulid(recording_id));
			qb.push_bind(values.0);
			qb.push_bind(values.1);
			qb.push_bind(common::database::Ulid(Ulid::new()));
			qb.push_bind(values.2);
			qb.push_bind(values.3);
		});

		qb.push(" RETURNING *");

		results.extend(qb.build_query_as().fetch_all(global.db().as_ref()).await.unwrap());
	}

	results
}

pub async fn create_recording_config(
	global: &Arc<GlobalState>,
	organization_id: Ulid,
	s3_bucket_id: Ulid,
	tags: HashMap<String, String>,
) -> video_common::database::RecordingConfig {
	sqlx::query_as(
		"INSERT INTO recording_configs (id, organization_id, s3_bucket_id, tags) VALUES ($1, $2, $3, $4) RETURNING *",
	)
	.bind(common::database::Ulid(Ulid::new()))
	.bind(common::database::Ulid(organization_id))
	.bind(common::database::Ulid(s3_bucket_id))
	.bind(sqlx::types::Json(tags))
	.fetch_one(global.db().as_ref())
	.await
	.unwrap()
}

pub async fn create_transcoding_config(
	global: &Arc<GlobalState>,
	organization_id: Ulid,
	tags: HashMap<String, String>,
) -> video_common::database::TranscodingConfig {
	sqlx::query_as("INSERT INTO transcoding_configs (id, organization_id, tags) VALUES ($1, $2, $3) RETURNING *")
		.bind(common::database::Ulid(Ulid::new()))
		.bind(common::database::Ulid(organization_id))
		.bind(sqlx::types::Json(tags))
		.fetch_one(global.db().as_ref())
		.await
		.unwrap()
}

pub async fn create_s3_bucket(
	global: &Arc<GlobalState>,
	organization_id: Ulid,
	tags: HashMap<String, String>,
) -> video_common::database::S3Bucket {
	sqlx::query_as(
        "INSERT INTO s3_buckets (id, organization_id, name, region, access_key_id, secret_access_key, managed, tags) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *",
    )
    .bind(common::database::Ulid(Ulid::new()))
    .bind(common::database::Ulid(organization_id))
    .bind("test".to_string())
    .bind("us-east-1".to_string())
    .bind("test".to_string())
    .bind("test".to_string())
    .bind(false)
	.bind(sqlx::types::Json(tags))
    .fetch_one(global.db().as_ref())
    .await
    .unwrap()
}

pub async fn create_playback_keypair(
	global: &Arc<GlobalState>,
	organization_id: Ulid,
	tags: HashMap<String, String>,
) -> video_common::database::PlaybackKeyPair {
	let (key, fingerprint) = validate_public_key(include_str!("../certs/ec384/public.pem")).unwrap();

	sqlx::query_as(
		"INSERT INTO playback_key_pairs (id, organization_id, public_key, fingerprint, updated_at, tags) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *",
	)
	.bind(common::database::Ulid(Ulid::new()))
	.bind(common::database::Ulid(organization_id))
	.bind(key)
	.bind(fingerprint)
	.bind(chrono::Utc::now())
	.bind(sqlx::types::Json(tags))
	.fetch_one(global.db().as_ref())
	.await
	.unwrap()
}
