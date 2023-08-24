use std::sync::Arc;

use pb::scuffle::video::v1::types::{
    AudioConfig, RecordingConfig, Rendition, TranscodingConfig, VideoConfig,
};
use prost::Message;
use ulid::Ulid;
use uuid::Uuid;
use video_database::room::Room;

use crate::{global::GlobalState, transcoder::job::renditions::determine_output_renditions};

pub struct SqlOperations {
    pub transcoding_config: TranscodingConfig,
    pub recording_config: Option<RecordingConfig>,
    pub video_input: VideoConfig,
    pub audio_input: AudioConfig,
    pub video_output: Vec<VideoConfig>,
    pub audio_output: Vec<AudioConfig>,
}

pub async fn perform_sql_operations(
    global: &Arc<GlobalState>,
    organization_id: Ulid,
    room_id: Ulid,
    connection_id: Ulid,
) -> anyhow::Result<SqlOperations> {
    let room: Option<Room> = match sqlx::query_as(
        r#"
        SELECT 
            *
        FROM rooms
        WHERE
            organization_id = $1 AND
            id = $2 AND
            active_ingest_connection_id = $3
        "#,
    )
    .bind(Uuid::from(organization_id))
    .bind(Uuid::from(room_id))
    .bind(Uuid::from(connection_id))
    .fetch_optional(global.db.as_ref())
    .await
    {
        Ok(r) => r,
        Err(err) => {
            anyhow::bail!("failed to query room: {}", err);
        }
    };

    let Some(room) = room else {
        anyhow::bail!("room not found");
    };

    let Some(video_input) = room.video_input else {
        anyhow::bail!("room has no video input");
    };
    let video_input = video_input.0;

    let Some(audio_input) = room.audio_input else {
        anyhow::bail!("room has no audio input");
    };
    let audio_input = audio_input.0;

    let recording_config = if let Some(recording_config) = room.active_recording_config {
        Some(recording_config.0)
    } else if let Some(recording_config_id) = &room.recording_config_id {
        Some(
            match sqlx::query_as::<_, video_database::recording_config::RecordingConfig>(
                "SELECT * FROM recording_configs WHERE organization_id = $1 AND id = $2",
            )
            .bind(Uuid::from(organization_id))
            .bind(recording_config_id)
            .fetch_one(global.db.as_ref())
            .await
            {
                Ok(r) => r.into_proto(),
                Err(err) => {
                    anyhow::bail!("failed to query recording config: {}", err);
                }
            },
        )
    } else {
        None
    };

    let transcoding_config = if let Some(transcoding_config) = room.active_transcoding_config {
        transcoding_config.0
    } else if let Some(transcoding_config_id) = &room.transcoding_config_id {
        match sqlx::query_as::<_, video_database::transcoding_config::TranscodingConfig>(
            "SELECT * FROM transcoding_configs WHERE organization_id = $1 AND id = $2",
        )
        .bind(Uuid::from(organization_id))
        .bind(*transcoding_config_id)
        .fetch_one(global.db.as_ref())
        .await
        {
            Ok(r) => r.into_proto(),
            Err(err) => {
                anyhow::bail!("failed to query transcoding config: {}", err);
            }
        }
    } else {
        TranscodingConfig {
            renditions: vec![Rendition::AudioSource.into(), Rendition::VideoSource.into()],
            ..Default::default()
        }
    };

    let (video_output, audio_output) =
        determine_output_renditions(&video_input, &audio_input, &transcoding_config);

    sqlx::query(
        r#"
        UPDATE rooms
        SET
            updated_at = NOW(),
            active_transcoding_config = $1,
            active_recording_config = $2,
            video_output = $3,
            audio_output = $4
        WHERE 
            organization_id = $5 AND
            id = $6 AND
            active_ingest_connection_id = $7
    "#,
    )
    .bind(transcoding_config.encode_to_vec())
    .bind(recording_config.as_ref().map(|v| v.encode_to_vec()))
    .bind(
        video_output
            .iter()
            .map(|v| v.encode_to_vec())
            .collect::<Vec<_>>(),
    )
    .bind(
        audio_output
            .iter()
            .map(|v| v.encode_to_vec())
            .collect::<Vec<_>>(),
    )
    .bind(Uuid::from(organization_id))
    .bind(Uuid::from(room_id))
    .bind(Uuid::from(connection_id))
    .execute(global.db.as_ref())
    .await?;

    Ok(SqlOperations {
        transcoding_config,
        recording_config,
        video_input,
        audio_input,
        video_output,
        audio_output,
    })
}
