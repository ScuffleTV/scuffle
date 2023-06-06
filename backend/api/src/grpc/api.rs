use crate::global::GlobalState;
use std::sync::{Arc, Weak};

use crate::database::{
    global_role,
    stream::{self, ReadyState},
    stream_event,
};
use chrono::{Duration, TimeZone, Utc};
use prost::Message;
use tonic::{async_trait, Request, Response, Status};
use uuid::Uuid;

use crate::pb::scuffle::backend::{
    api_server,
    update_live_stream_request::{event::Level, update::Update},
    AuthenticateLiveStreamRequest, AuthenticateLiveStreamResponse, NewLiveStreamRequest,
    NewLiveStreamResponse, StreamReadyState, UpdateLiveStreamRequest, UpdateLiveStreamResponse,
};

type Result<T> = std::result::Result<T, Status>;

pub struct ApiServer {
    global: Weak<GlobalState>,
}

impl ApiServer {
    pub fn new(global: &Arc<GlobalState>) -> Self {
        Self {
            global: Arc::downgrade(global),
        }
    }

    pub fn into_service(self) -> api_server::ApiServer<Self> {
        api_server::ApiServer::new(self)
    }
}
#[async_trait]
impl api_server::Api for ApiServer {
    async fn authenticate_live_stream(
        &self,
        request: Request<AuthenticateLiveStreamRequest>,
    ) -> Result<Response<AuthenticateLiveStreamResponse>> {
        let global = self
            .global
            .upgrade()
            .ok_or_else(|| Status::internal("internal server error"))?;

        // Split the stream key into components
        let request = request.into_inner();

        let components = request.stream_key.split('_').collect::<Vec<_>>();
        if components.len() != 3 {
            return Err(Status::invalid_argument("invalid stream key"));
        }

        let (live, channel_id, stream_key) = (
            components[0].to_string(),
            components[1].to_string(),
            components[2].to_string(),
        );

        if live != "live" {
            return Err(Status::invalid_argument("invalid stream key"));
        }

        let channel_id = Uuid::from_u128(
            channel_id
                .parse::<u128>()
                .map_err(|_| Status::invalid_argument("invalid stream key"))?,
        );

        let channel = global
            .user_by_id_loader
            .load_one(channel_id)
            .await
            .map_err(|_| Status::internal("failed to query database"))?
            .ok_or_else(|| Status::invalid_argument("invalid stream key"))?;

        if channel.stream_key != stream_key {
            return Err(Status::invalid_argument(
                "invalid stream key: incorrect stream key",
            ));
        }

        // Check user permissions
        let Ok(permissions) = global.user_permisions_by_id_loader.load_one(channel_id).await else {
            return Err(Status::internal("failed to query database"));
        };

        let Some(user_permissions) = permissions else {
            return Err(Status::permission_denied("user has no permission to go live"));
        };

        if !user_permissions
            .permissions
            .has_permission(global_role::Permission::GoLive)
        {
            return Err(Status::permission_denied(
                "user has no permission to go live",
            ));
        }

        // We need to create a new stream ID for this stream
        let mut tx = global.db.begin().await.map_err(|e| {
            tracing::error!("failed to begin transaction: {}", e);
            Status::internal("internal server error")
        })?;

        let record = user_permissions
            .permissions
            .has_permission(global_role::Permission::StreamRecording)
            && channel.stream_recording_enabled;
        let transcode = user_permissions
            .permissions
            .has_permission(global_role::Permission::StreamTranscoding)
            && channel.stream_transcoding_enabled;

        let stream = match sqlx::query_as!(
            stream::Model,
            "INSERT INTO streams (channel_id, title, description, recorded, transcoded, ingest_address, connection_id, ended_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *",
            channel_id,
            channel.stream_title,
            channel.stream_description,
            record,
            transcode,
            request.ingest_address,
            request.connection_id.parse::<Uuid>().map_err(|_| Status::invalid_argument("invalid connection ID: must be a valid UUID"))?,
            Utc::now() + chrono::Duration::seconds(300),
        ).fetch_one(&mut *tx).await {
            Ok(stream) => stream,
            Err(e) => {
                tracing::error!("failed to insert stream: {}", e);
                return Err(Status::internal("internal server error"));
            }
        };

        if let Err(e) = tx.commit().await {
            tracing::error!("failed to commit transaction: {}", e);
            return Err(Status::internal("internal server error"));
        }

        Ok(Response::new(AuthenticateLiveStreamResponse {
            stream_id: stream.id.to_string(),
            record,
            transcode,
            state: None,
        }))
    }

    async fn update_live_stream(
        &self,
        request: Request<UpdateLiveStreamRequest>,
    ) -> Result<Response<UpdateLiveStreamResponse>> {
        let global = self
            .global
            .upgrade()
            .ok_or_else(|| Status::internal("internal server error"))?;

        let request = request.into_inner();

        let stream_id = request
            .stream_id
            .parse::<Uuid>()
            .map_err(|_| Status::invalid_argument("invalid stream ID: must be a valid UUID"))?;

        let connection_id = request
            .connection_id
            .parse::<Uuid>()
            .map_err(|_| Status::invalid_argument("invalid connection ID: must be a valid UUID"))?;

        if request.updates.is_empty() {
            return Err(Status::invalid_argument("no updates provided"));
        }

        let stream = global
            .stream_by_id_loader
            .load_one(stream_id)
            .await
            .map_err(|_| Status::internal("failed to query database"))?
            .ok_or_else(|| Status::invalid_argument("invalid stream ID"))?;

        if stream.connection_id != connection_id {
            return Err(Status::invalid_argument("invalid connection ID"));
        }

        if stream.ended_at < Utc::now()
            || stream.ready_state == ReadyState::Stopped
            || stream.ready_state == ReadyState::Failed
            || stream.ready_state == ReadyState::StoppedResumable
        {
            return Err(Status::invalid_argument("stream has ended"));
        }

        let mut tx = global.db.begin().await.map_err(|e| {
            tracing::error!("failed to begin transaction: {}", e);
            Status::internal("internal server error")
        })?;

        for u in request.updates {
            let Some(update) = u.update else {
                continue;
            };

            match update {
                Update::Bitrate(bt) => {
                    sqlx::query!(
                        "INSERT INTO stream_bitrate_updates (stream_id, video_bitrate, audio_bitrate, metadata_bitrate, created_at) VALUES ($1, $2, $3, $4, $5)",
                        stream_id,
                        bt.video_bitrate as i64,
                        bt.audio_bitrate as i64,
                        bt.metadata_bitrate as i64,
                        Utc.timestamp_opt(u.timestamp as i64, 0).unwrap(),
                    ).execute(&mut *tx).await.map_err(|e| {
                        tracing::error!("failed to insert stream bitrate update: {}", e);
                        Status::internal("internal server error")
                    })?;

                    sqlx::query!(
                        "UPDATE streams SET updated_at = $2, ended_at = $3 WHERE id = $1",
                        stream_id,
                        Utc.timestamp_opt(u.timestamp as i64, 0).unwrap(),
                        Utc.timestamp_opt(u.timestamp as i64, 0).unwrap()
                            + chrono::Duration::seconds(300),
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| {
                        tracing::error!("failed to insert stream bitrate update: {}", e);
                        Status::internal("internal server error")
                    })?;
                }
                Update::ReadyState(st) => {
                    let state = StreamReadyState::from_i32(st).ok_or_else(|| {
                        Status::invalid_argument("invalid ready_state: must be a valid ready_state")
                    })?;
                    match state {
                        StreamReadyState::NotReady | StreamReadyState::Ready => {
                            sqlx::query!(
                                "UPDATE streams SET ready_state = $2, updated_at = $3, ended_at = $4 WHERE id = $1",
                                stream_id,
                                match state {
                                    StreamReadyState::NotReady => ReadyState::NotReady as i64,
                                    StreamReadyState::Ready => ReadyState::Ready as i64,
                                    _ => unreachable!(),
                                },
                                Utc.timestamp_opt(u.timestamp as i64, 0).unwrap(),
                                Utc.timestamp_opt(u.timestamp as i64, 0).unwrap() + chrono::Duration::seconds(300),
                            )
                            .execute(&*global.db)
                            .await
                            .map_err(|e| {
                                tracing::error!("failed to update stream state: {}", e);
                                Status::internal("internal server error")
                            })?;
                        }
                        StreamReadyState::StoppedResumable => {
                            sqlx::query!(
                                "UPDATE streams SET ready_state = $2, updated_at = $3, ended_at = $4 WHERE id = $1",
                                stream_id,
                                ReadyState::StoppedResumable as i64,
                                Utc.timestamp_opt(u.timestamp as i64, 0).unwrap(),
                                Utc.timestamp_opt(u.timestamp as i64, 0).unwrap() + Duration::seconds(300),
                            ).execute(&*global.db).await.map_err(|e| {
                                tracing::error!("failed to update stream state: {}", e);
                                Status::internal("internal server error")
                            })?;
                        }
                        StreamReadyState::Stopped | StreamReadyState::Failed => {
                            sqlx::query!(
                                "UPDATE streams SET ready_state = $2, updated_at = $3, ended_at = $3 WHERE id = $1",
                                stream_id,
                                match state {
                                    StreamReadyState::Stopped => ReadyState::Stopped as i64,
                                    StreamReadyState::Failed => ReadyState::Failed as i64,
                                    _ => unreachable!(),
                                },
                                Utc.timestamp_opt(u.timestamp as i64, 0).unwrap()
                            )
                            .execute(&*global.db)
                            .await
                            .map_err(|e| {
                                tracing::error!("failed to update stream state: {}", e);
                                Status::internal("internal server error")
                            })?;
                        }
                    }
                }
                Update::Event(e) => {
                    let level = Level::from_i32(e.level).ok_or_else(|| {
                        Status::invalid_argument("invalid level: must be a valid level")
                    })?;
                    let level = match level {
                        Level::Info => stream_event::Level::Info,
                        Level::Warning => stream_event::Level::Warning,
                        Level::Error => stream_event::Level::Error,
                    };

                    sqlx::query!(
                        "INSERT INTO stream_events (stream_id, level, title, message, created_at) VALUES ($1, $2, $3, $4, $5)",
                        stream_id,
                        level as i64,
                        e.title,
                        e.message,
                        Utc.timestamp_opt(u.timestamp as i64, 0).unwrap(),
                    ).execute(&mut *tx).await.map_err(|e| {
                        tracing::error!("failed to insert stream event: {}", e);
                        Status::internal("internal server error")
                    })?;
                }
                Update::State(v) => {
                    sqlx::query!(
                        "UPDATE streams SET updated_at = NOW(), state = $2 WHERE id = $1",
                        stream_id,
                        v.encode_to_vec(),
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| {
                        tracing::error!("failed to insert stream bitrate update: {}", e);
                        Status::internal("internal server error")
                    })?;
                }
            }
        }

        if let Err(e) = tx.commit().await {
            tracing::error!("failed to commit transaction: {}", e);
            return Err(Status::internal("internal server error"));
        }

        Ok(Response::new(UpdateLiveStreamResponse {}))
    }

    async fn new_live_stream(
        &self,
        request: Request<NewLiveStreamRequest>,
    ) -> Result<Response<NewLiveStreamResponse>> {
        let global = self
            .global
            .upgrade()
            .ok_or_else(|| Status::internal("internal server error"))?;

        let request = request.into_inner();

        let old_stream_id = request
            .old_stream_id
            .parse::<Uuid>()
            .map_err(|_| Status::invalid_argument("invalid old stream ID: must be a valid UUID"))?;

        let Some(old_stream) = global.stream_by_id_loader.load_one(old_stream_id).await.map_err(|e| {
            tracing::error!("failed to load stream by ID: {}", e);
            Status::internal("internal server error")
        })? else {
            return Err(Status::not_found("stream not found"));
        };

        if old_stream.ended_at < Utc::now()
            || old_stream.ready_state == ReadyState::Stopped
            || old_stream.ready_state == ReadyState::Failed
        {
            return Err(Status::failed_precondition("stream has already ended"));
        }

        let stream_id = Uuid::new_v4();

        let mut tx = global.db.begin().await.map_err(|e| {
            tracing::error!("failed to begin transaction: {}", e);
            Status::internal("internal server error")
        })?;

        // Insert the new stream
        sqlx::query!(
            "INSERT INTO streams (id, channel_id, title, description, ready_state, ingest_address, connection_id) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            stream_id,
            old_stream.channel_id,
            old_stream.title,
            old_stream.description,
            ReadyState::NotReady as i64,
            old_stream.ingest_address,
            old_stream.connection_id,
        ).execute(&mut *tx).await.map_err(|e| {
            tracing::error!("failed to insert stream: {}", e);
            Status::internal("internal server error")
        })?;

        // Update the old stream
        sqlx::query!(
            "UPDATE streams SET ready_state = $2, ended_at = NOW(), updated_at = NOW() WHERE id = $1",
            old_stream_id,
            ReadyState::Stopped as i32,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("failed to update stream: {}", e);
            Status::internal("internal server error")
        })?;

        sqlx::query!(
            "UPDATE streams SET updated_at = NOW(), state = $2 WHERE id = $1",
            stream_id,
            request.state.unwrap_or_default().encode_to_vec(),
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("failed to insert stream bitrate update: {}", e);
            Status::internal("internal server error")
        })?;

        if let Err(e) = tx.commit().await {
            tracing::error!("failed to commit transaction: {}", e);
            return Err(Status::internal("internal server error"));
        }

        Ok(Response::new(NewLiveStreamResponse {
            stream_id: stream_id.to_string(),
        }))
    }
}
