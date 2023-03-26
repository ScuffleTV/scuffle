use crate::global::GlobalState;
use std::sync::{Arc, Weak};

use crate::database::{
    global_role,
    stream::{self, State},
    stream_event, stream_variant,
};
use chrono::{Duration, TimeZone, Utc};
use sqlx::{Executor, Postgres, QueryBuilder};
use tonic::{async_trait, Request, Response, Status};
use uuid::Uuid;

use super::pb::scuffle::{
    backend::{
        api_server,
        update_live_stream_request::{event::Level, update::Update},
        AuthenticateLiveStreamRequest, AuthenticateLiveStreamResponse, LiveStreamState,
        NewLiveStreamRequest, NewLiveStreamResponse, UpdateLiveStreamRequest,
        UpdateLiveStreamResponse,
    },
    types::StreamVariant,
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

    async fn insert_stream_variants<'c, T: Executor<'c, Database = Postgres>>(
        tx: T,
        stream_id: Uuid,
        variants: &Vec<StreamVariant>,
    ) -> Result<()> {
        // Insert the new stream variants
        let mut values = Vec::new();

        // Unfortunately, we can't use the `sqlx::query!` macro here because it doesn't support
        // batch inserts. So we have to build the query manually. This is a bit of a pain, because
        // the query is not compile time checked, but it's better than nothing.
        let mut query_builder = QueryBuilder::new(
            "
        INSERT INTO stream_variants (
            id,
            stream_id,
            name,
            video_framerate,
            video_height,
            video_width,
            video_bitrate,
            video_codec,
            audio_bitrate,
            audio_channels,
            audio_sample_rate,
            audio_codec,
            metadata,
            created_at
        ) ",
        );

        for variant in variants {
            let variant_id = variant.id.parse::<Uuid>().map_err(|_| {
                Status::invalid_argument("invalid variant ID: must be a valid UUID")
            })?;

            values.push(stream_variant::Model {
                id: variant_id,
                stream_id,
                name: variant.name.clone(),
                video_framerate: variant.video_settings.as_ref().map(|v| v.framerate as i64),
                video_height: variant.video_settings.as_ref().map(|v| v.height as i64),
                video_width: variant.video_settings.as_ref().map(|v| v.width as i64),
                video_bitrate: variant.video_settings.as_ref().map(|v| v.bitrate as i64),
                video_codec: variant.video_settings.as_ref().map(|v| v.codec.clone()),
                audio_bitrate: variant.audio_settings.as_ref().map(|a| a.bitrate as i64),
                audio_channels: variant.audio_settings.as_ref().map(|a| a.channels as i64),
                audio_sample_rate: variant
                    .audio_settings
                    .as_ref()
                    .map(|a| a.sample_rate as i64),
                audio_codec: variant.audio_settings.as_ref().map(|a| a.codec.clone()),
                metadata: serde_json::from_str(&variant.metadata).unwrap_or_default(),
                created_at: Utc::now(),
            })
        }

        query_builder.push_values(values, |mut b, variant| {
            b.push_bind(variant.id)
                .push_bind(variant.stream_id)
                .push_bind(variant.name)
                .push_bind(variant.video_framerate)
                .push_bind(variant.video_height)
                .push_bind(variant.video_width)
                .push_bind(variant.video_bitrate)
                .push_bind(variant.video_codec)
                .push_bind(variant.audio_bitrate)
                .push_bind(variant.audio_channels)
                .push_bind(variant.audio_sample_rate)
                .push_bind(variant.audio_codec)
                .push_bind(variant.metadata)
                .push_bind(variant.created_at);
        });

        query_builder.build().execute(tx).await.map_err(|e| {
            tracing::error!("failed to insert stream variants: {}", e);
            Status::internal("internal server error")
        })?;

        Ok(())
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
            try_resume: false,
            variants: vec![],
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
            || stream.state == State::Stopped
            || stream.state == State::Failed
            || stream.state == State::StoppedResumable
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
                Update::State(st) => {
                    let state = LiveStreamState::from_i32(st).ok_or_else(|| {
                        Status::invalid_argument("invalid state: must be a valid state")
                    })?;
                    match state {
                        LiveStreamState::NotReady | LiveStreamState::Ready => {
                            sqlx::query!(
                                "UPDATE streams SET state = $2, updated_at = $3, ended_at = $4 WHERE id = $1",
                                stream_id,
                                match state {
                                    LiveStreamState::NotReady => State::NotReady as i64,
                                    LiveStreamState::Ready => State::Ready as i64,
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
                        LiveStreamState::StoppedResumable => {
                            sqlx::query!(
                                "UPDATE streams SET state = $2, updated_at = $3, ended_at = $4 WHERE id = $1",
                                stream_id,
                                State::StoppedResumable as i64,
                                Utc.timestamp_opt(u.timestamp as i64, 0).unwrap(),
                                Utc.timestamp_opt(u.timestamp as i64, 0).unwrap() + Duration::seconds(300),
                            ).execute(&*global.db).await.map_err(|e| {
                                tracing::error!("failed to update stream state: {}", e);
                                Status::internal("internal server error")
                            })?;
                        }
                        LiveStreamState::Stopped | LiveStreamState::Failed => {
                            sqlx::query!(
                                "UPDATE streams SET state = $2, updated_at = $3, ended_at = $3 WHERE id = $1",
                                stream_id,
                                match state {
                                    LiveStreamState::Stopped => State::Stopped as i64,
                                    LiveStreamState::Failed => State::Failed as i64,
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
                Update::Variants(v) => {
                    ApiServer::insert_stream_variants(&mut *tx, stream_id, &v.variants).await?;

                    sqlx::query!(
                        "UPDATE streams SET updated_at = NOW() WHERE id = $1",
                        stream_id,
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
            || old_stream.state == State::Stopped
            || old_stream.state == State::Failed
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
            "INSERT INTO streams (id, channel_id, title, description, state, ingest_address, connection_id) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            stream_id,
            old_stream.channel_id,
            old_stream.title,
            old_stream.description,
            State::NotReady as i64,
            old_stream.ingest_address,
            old_stream.connection_id,
        ).execute(&mut *tx).await.map_err(|e| {
            tracing::error!("failed to insert stream: {}", e);
            Status::internal("internal server error")
        })?;

        // Update the old stream
        sqlx::query!(
            "UPDATE streams SET state = $2, ended_at = NOW(), updated_at = NOW() WHERE id = $1",
            old_stream_id,
            State::Stopped as i32,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("failed to update stream: {}", e);
            Status::internal("internal server error")
        })?;

        ApiServer::insert_stream_variants(&mut *tx, stream_id, &request.variants).await?;

        sqlx::query!(
            "UPDATE streams SET updated_at = NOW() WHERE id = $1",
            stream_id,
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

        // let r = global
        //     .nats
        //     .publish(
        //         old_stream.events_subject,
        //         events::IngestMessage {
        //             id: Uuid::new_v4().to_string(),
        //             timestamp: Utc::now().timestamp() as u64,
        //             data: Some(events::ingest_message::Data::DropStream(
        //                 events::IngestMessageDropStream {
        //                     id: old_stream_id.to_string(),
        //                 },
        //             )),
        //         }
        //         .encode_to_vec()
        //         .into(),
        //     )
        //     .await;
        // if let Err(e) = r {
        //     tracing::error!("failed to publish NATS message: {}", e);
        // }

        Ok(Response::new(NewLiveStreamResponse {
            stream_id: stream_id.to_string(),
        }))
    }
}
