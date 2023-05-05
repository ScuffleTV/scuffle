use bytes::Bytes;
use bytesio::bytesio::AsyncReadWrite;
use chrono::Utc;
use common::prelude::FutureTimeout;
use flv::{FlvTag, FlvTagData, FlvTagType};
use futures::Future;
use lapin::{options::BasicPublishOptions, BasicProperties};
use prost::Message as _;
use rtmp::{ChannelData, DataConsumer, PublishRequest, Session, SessionError};
use std::{collections::HashMap, net::IpAddr, pin::pin, sync::Arc, time::Duration};
use tokio::{
    select,
    sync::{broadcast, mpsc},
    time::Instant,
};
use tonic::{transport::Channel, Code};
use transmuxer::{AudioSettings, MediaSegment, TransmuxResult, Transmuxer, VideoSettings};
use uuid::Uuid;

use crate::{
    connection_manager::{GrpcRequest, WatchStreamEvent},
    global::GlobalState,
    ingest::variants::generate_variants,
    pb::scuffle::{
        backend::{
            api_client::ApiClient,
            update_live_stream_request::{self, event, update, Bitrate, Event, Update},
            AuthenticateLiveStreamRequest, LiveStreamState, NewLiveStreamRequest,
            UpdateLiveStreamRequest,
        },
        events::{self, transcoder_message},
        types::StreamVariant,
    },
};

struct Connection {
    id: Uuid,
    api_resp: ApiResponse,
    data_reciever: DataConsumer,
    transmuxer: Transmuxer,
    total_video_bytes: u64,
    total_audio_bytes: u64,
    total_metadata_bytes: u64,

    bytes_since_keyframe: u64,

    api_client: ApiClient<Channel>,
    stream_id_sender: broadcast::Sender<Uuid>,
    transcoder_req_rx: mpsc::Receiver<GrpcRequest>,

    initial_segment: Option<Bytes>,
    fragment_list: Vec<MediaSegment>,

    current_transcoder: Option<mpsc::Sender<WatchStreamEvent>>, // The current main transcoder
    current_transcoder_id: Option<Uuid>,                        // The current main transcoder id

    next_transcoder: Option<mpsc::Sender<WatchStreamEvent>>, // The next transcoder to be used
    next_transcoder_id: Option<Uuid>,                        // The next transcoder to be used

    last_transcoder_publish: Instant,

    report_shutdown: bool,

    transcoder_req_tx: mpsc::Sender<GrpcRequest>,
}

#[derive(Default)]
struct ApiResponse {
    id: Uuid,
    transcode: bool,
    record: bool,
    try_resume: bool,
    variants: Vec<StreamVariant>,
}

const BITRATE_UPDATE_INTERVAL: u64 = 5;
const MAX_TRANSCODER_WAIT_TIME: u64 = 60;
const MAX_BITRATE: u64 = 16000 * 1024; // 16000kbps
const MAX_BYTES_BETWEEN_KEYFRAMES: u64 = MAX_BITRATE * 4 / 8; // 4 seconds of video at max bitrate (ie. 4 seconds between keyframes) which is ~12MB

async fn update_api(
    connection_id: Uuid,
    mut update_reciever: mpsc::Receiver<Vec<Update>>,
    mut api_client: ApiClient<Channel>,
    mut stream_id: broadcast::Receiver<Uuid>,
) {
    let Ok(stream_id) = stream_id.recv().await else {
        return;
    };

    while let Some(updates) = update_reciever.recv().await {
        let mut success = false;
        for _ in 0..5 {
            if let Err(e) = api_client
                .update_live_stream(UpdateLiveStreamRequest {
                    connection_id: connection_id.to_string(),
                    stream_id: stream_id.to_string(),
                    updates: updates.clone(),
                })
                .await
            {
                tracing::error!(msg = e.message(), status = ?e.code(), "api grpc error");
                tokio::time::sleep(Duration::from_secs(1)).await;
            } else {
                success = true;
                break;
            }
        }

        if !success {
            tracing::error!("failed to update api with bitrate after 5 retries - giving up");
            return;
        }
    }
}

#[tracing::instrument(skip(global, socket))]
pub async fn handle<S: AsyncReadWrite>(global: Arc<GlobalState>, socket: S, ip: IpAddr) {
    // We only need a single buffer channel for this session because the entire session is single threaded
    // and we don't need to worry about buffering.
    let (event_producer, mut event_reciever) = mpsc::channel(1);
    let (data_producer, data_reciever) = mpsc::channel(1);

    let mut session = Session::new(socket, data_producer, event_producer);

    // When a future is pinned it becomes pausable and can be resumed later
    // The entire design here is to run on a single task, and share execution on the single thread.
    // So when we select on this future we are allowing this future to execute.
    // This makes it so the session cannot run outside of its turn.
    // Essentially this is how tokio's executor works, but we are doing it manually.
    // This also has the advantage of being completely cleaned up when the function goes out of scope.
    // If we used a tokio::spawn here, we would have to manually clean up the task.
    let mut session_fut = pin!(session.run());

    let event;

    select! {
        _ = global.ctx.done() => {
            tracing::debug!("Global context closed, closing connection");
            return;
        },
        _ = &mut session_fut => {
            tracing::debug!("session closed before publish request");
            return;
        },
        _ = tokio::time::sleep(Duration::from_secs(5)) => {
            tracing::debug!("session timed out before publish request");
            return;
        },
        e = event_reciever.recv() => {
            event = e.expect("event producer closed");
        },
    };

    let (transcoder_req_tx, transcoder_req_rx) = mpsc::channel(128);

    let mut connection = Connection {
        id: Uuid::new_v4(), // Unique ID for this connection
        api_resp: ApiResponse::default(),
        data_reciever,
        transmuxer: Transmuxer::new(),
        total_audio_bytes: 0,
        total_metadata_bytes: 0,
        total_video_bytes: 0,
        api_client: global.api_client(),
        stream_id_sender: broadcast::channel(1).0,
        transcoder_req_rx,
        transcoder_req_tx,
        current_transcoder: None,
        next_transcoder: None,
        initial_segment: None,
        fragment_list: Vec::new(),
        last_transcoder_publish: Instant::now(),
        current_transcoder_id: None,
        next_transcoder_id: None,
        report_shutdown: true,
        bytes_since_keyframe: 0,
    };

    if connection.request_api(&global, event, ip).await {
        connection.run(global, session_fut).await;
    }
}

impl Connection {
    #[tracing::instrument(
        level = "debug",
        skip(self, global, event, ip),
        fields(app = %event.app_name, stream = %event.stream_name)
    )]
    async fn request_api(
        &mut self,
        global: &Arc<GlobalState>,
        event: PublishRequest,
        ip: IpAddr,
    ) -> bool {
        let response = self
            .api_client
            .authenticate_live_stream(AuthenticateLiveStreamRequest {
                app_name: event.app_name.clone(),
                stream_key: event.stream_name.clone(),
                ip_address: ip.to_string(),
                ingest_address: global.config.grpc.advertise_address.clone(),
                connection_id: self.id.to_string(),
            })
            .await;

        let response = match response {
            Ok(r) => r.into_inner(),
            Err(e) => {
                match e.code() {
                    Code::PermissionDenied => {
                        tracing::debug!(msg = e.message(), "api denied publish request")
                    }
                    Code::InvalidArgument => {
                        tracing::debug!(msg = e.message(), "api rejected publish request")
                    }
                    _ => {
                        tracing::error!(msg = e.message(), status = ?e.code(), "api grpc error");
                    }
                }
                return false;
            }
        };

        let Ok(id) = Uuid::parse_str(&response.stream_id) else {
            tracing::error!("api responded with bad uuid: {}", response.stream_id);
            return false;
        };

        if event.response.send(id).is_err() {
            tracing::warn!("publish request receiver closed");
            return false;
        }

        self.api_resp = ApiResponse {
            id,
            transcode: response.transcode,
            record: response.record,
            try_resume: response.try_resume,
            variants: response.variants,
        };

        true
    }

    #[tracing::instrument(
        level = "info",
        skip(self, global, session_fut),
        fields(id = %self.api_resp.id, transcode = self.api_resp.transcode, record = self.api_resp.record)
    )]
    async fn run<F: Future<Output = Result<bool, SessionError>> + Send + Unpin>(
        &mut self,
        global: Arc<GlobalState>,
        session_fut: F,
    ) {
        tracing::info!("new publish request");

        // At this point we have a stream that is publishing to us
        // We can now poll the run future & the data receiver.
        // The run future will close when the connection is closed or an error occurs
        // The data receiver will never close, because the Session object is always in scope.

        let mut bitrate_update_interval = tokio::time::interval(Duration::from_secs(5));
        bitrate_update_interval.tick().await; // Skip the first tick (resolves instantly)

        let (update_channel, update_reciever) = mpsc::channel(10);

        let mut session_fut = session_fut;

        let mut api_update_fut = pin!(update_api(
            self.id,
            update_reciever,
            self.api_client.clone(),
            self.stream_id_sender.subscribe()
        ));

        let mut next_timeout = Instant::now() + Duration::from_secs(2);

        let mut clean_shutdown = false;
        // We need to keep track of whether the api update failed, so we can
        // not poll it again if its finished. (this will panic if we poll it again)
        let mut api_update_failed = false;

        while select! {
            _ = global.ctx.done() => {
                tracing::debug!("Global context closed, closing connection");

                false
            },
            r = &mut session_fut => {
                tracing::debug!("session closed before publish request");
                match r {
                    Ok(clean) => clean_shutdown = clean,
                    Err(e) => tracing::error!("Connection error: {}", e),
                }

                false
            },
            data = self.data_reciever.recv() => {
                next_timeout = Instant::now() + Duration::from_secs(2);
                self.on_data(&update_channel, &global, data.expect("data producer closed")).await
            },
            _ = bitrate_update_interval.tick() => self.on_bitrate_update(&update_channel),
            _ = tokio::time::sleep_until(next_timeout) => {
                tracing::debug!("session timed out during data");
                false
            },
            _ = &mut api_update_fut => {
                tracing::error!("api update future failed");
                api_update_failed = true;
                false
            }
            event = self.transcoder_req_rx.recv() => self.on_transcoder_request(&update_channel, &global, event.expect("transcoder closed")).await,
        } {}

        if let Some(transcoder) = self.current_transcoder.take() {
            transcoder
                .send(WatchStreamEvent::ShuttingDown(true))
                .await
                .ok();
        }

        if let Some(transcoder) = self.next_transcoder.take() {
            transcoder
                .send(WatchStreamEvent::ShuttingDown(true))
                .await
                .ok();
        }

        if self.initial_segment.is_none() {
            self.stream_id_sender.send(self.api_resp.id).ok();
        }

        // Release the connection from the global state
        // if it was never stored in the first place, this will do nothing.
        global
            .connection_manager
            .deregister_stream(self.api_resp.id, self.id)
            .await;

        if self.report_shutdown && !api_update_failed {
            select! {
                r = update_channel.send(vec![Update {
                    timestamp: Utc::now().timestamp() as u64,
                    update: Some(update::Update::State(if clean_shutdown {
                        LiveStreamState::Stopped
                    } else {
                        LiveStreamState::StoppedResumable
                    } as i32)),
                }]) => {
                    if r.is_err() {
                        tracing::error!("api update channel blocked");
                    }
                },
                _ = &mut api_update_fut => {
                    tracing::error!("api update future failed");
                }
            }
        }

        drop(update_channel);

        if !api_update_failed {
            // Wait for the api update future to finish
            if api_update_fut
                .timeout(Duration::from_secs(5))
                .await
                .is_err()
            {
                tracing::error!("api update future timed out");
            }
        }

        tracing::info!(clean = clean_shutdown, "connection closed",);
    }

    async fn request_transcoder(
        &mut self,
        update_channel: &mpsc::Sender<Vec<Update>>,
        global: &Arc<GlobalState>,
    ) -> bool {
        // If we already have a request pending, then we don't need to request another one.
        if self.next_transcoder_id.is_some() {
            return true;
        }

        let request_id = Uuid::new_v4();
        self.next_transcoder_id = Some(request_id);

        let channel = match global.rmq.aquire().timeout(Duration::from_secs(1)).await {
            Ok(Ok(channel)) => channel,
            Ok(Err(e)) => {
                tracing::error!("failed to aquire channel: {}", e);
                return false;
            }
            Err(_) => {
                tracing::error!("failed to aquire channel: timed out");
                return false;
            }
        };

        if let Err(e) = channel
            .basic_publish(
                "",
                &global.config.transcoder.events_subject,
                BasicPublishOptions::default(),
                events::TranscoderMessage {
                    id: request_id.to_string(),
                    timestamp: Utc::now().timestamp() as u64,
                    data: Some(transcoder_message::Data::NewStream(
                        events::TranscoderMessageNewStream {
                            request_id: request_id.to_string(),
                            stream_id: self.api_resp.id.to_string(),
                            ingest_address: global.config.grpc.advertise_address.clone(),
                            variants: self.api_resp.variants.clone(),
                        },
                    )),
                }
                .encode_to_vec()
                .as_slice(),
                BasicProperties::default()
                    .with_message_id(request_id.to_string().into())
                    .with_content_type("application/octet-stream".into())
                    .with_expiration("60000".into()),
            )
            .await
        {
            tracing::error!("failed to publish to jetstream: {}", e);
            return false;
        }

        if update_channel
            .try_send(vec![Update {
                timestamp: Utc::now().timestamp() as u64,
                update: Some(update::Update::Event(Event {
                    title: "Requested Transcoder".to_string(),
                    message: "Requested a transcoder to be assigned to this stream".to_string(),
                    level: event::Level::Info as i32,
                })),
            }])
            .is_err()
        {
            tracing::error!("failed to send update to api");
            return false;
        }

        tracing::info!("requested transcoder");

        true
    }

    async fn on_transcoder_request(
        &mut self,
        update_channel: &mpsc::Sender<Vec<Update>>,
        global: &Arc<GlobalState>,
        req: GrpcRequest,
    ) -> bool {
        // There are 2 possible events that happen here, either we already have a transcoder in the current_transcoder field
        // Or we don't. If we do then we want to set this transcoder as the next transcoder, and when a keyframe is received
        // The state will be updated to the next transcoder.
        // If we don't have a transcoder, then we want to set the current transcoder and provide it with the data from the fragment list.

        let Some(init_segment) = &self.initial_segment else {
            return false;
        };

        match req {
            GrpcRequest::Started { id } => {
                tracing::info!("transcoder started: {}", id);
                if update_channel
                    .try_send(vec![Update {
                        timestamp: Utc::now().timestamp() as u64,
                        update: Some(update::Update::State(LiveStreamState::Ready as i32)),
                    }])
                    .is_err()
                {
                    tracing::error!("api update channel blocked");
                    return false;
                }
            }
            GrpcRequest::Error {
                id,
                message,
                fatal: _,
            } => {
                if self.current_transcoder_id == Some(id) || self.next_transcoder_id == Some(id) {
                    tracing::error!("transcoder failed: {}", message);

                    // When we report a state failed we dont need to report the shutdown to the API.
                    // This is because the API will already know that the stream has been dropped.
                    self.report_shutdown = false;

                    if update_channel
                        .try_send(vec![
                            Update {
                                timestamp: Utc::now().timestamp() as u64,
                                update: Some(update::Update::Event(Event {
                                    title: "Transcoder Error".to_string(),
                                    message,
                                    level: event::Level::Error as i32,
                                })),
                            },
                            Update {
                                timestamp: Utc::now().timestamp() as u64,
                                update: Some(update::Update::State(LiveStreamState::Failed as i32)),
                            },
                        ])
                        .is_err()
                    {
                        tracing::error!("api update channel blocked");
                        return false;
                    }

                    return false;
                } else {
                    tracing::warn!("transcoder request failure id mismatch");
                }
            }
            GrpcRequest::ShuttingDown { id } => {
                if self.current_transcoder_id == Some(id) {
                    tracing::info!("transcoder shutting down");
                    return self.request_transcoder(update_channel, global).await;
                } else if self.next_transcoder_id == Some(id) {
                    tracing::warn!("next transcoder shutting down");
                    if let Some(transcoder) = self.next_transcoder.take() {
                        transcoder
                            .send(WatchStreamEvent::ShuttingDown(false))
                            .await
                            .ok();
                    }
                    self.next_transcoder = None;
                    self.next_transcoder_id = None;
                    return self.request_transcoder(update_channel, global).await;
                } else {
                    tracing::warn!("transcoder request failure id mismatch");
                }
            }
            GrpcRequest::WatchStream { id, channel } => {
                if self.next_transcoder_id != Some(id) {
                    // This is a request for a transcoder that we don't care about.
                    tracing::warn!("transcoder request id mismatch");
                    return true;
                }

                if self.next_transcoder.is_some() {
                    // If this happens something has gone wrong, we should never have 3 transcoders.
                    tracing::warn!("new transcoder set while new transcoder is already pending");
                    return true;
                }

                if self.current_transcoder.is_some() || self.fragment_list.is_empty() {
                    if channel
                        .send(WatchStreamEvent::InitSegment(init_segment.clone()))
                        .await
                        .is_err()
                    {
                        // It seems the transcoder has already closed.
                        tracing::warn!("new transcoder closed during initialization");
                        return true;
                    }

                    self.next_transcoder = Some(channel);
                } else {
                    // We don't have a transcoder, so we can just set the current transcoder.
                    if channel
                        .send(WatchStreamEvent::InitSegment(init_segment.clone()))
                        .await
                        .is_err()
                    {
                        // It seems the transcoder has already closed.
                        tracing::warn!("transcoder closed during initialization");
                        return self.request_transcoder(update_channel, global).await;
                    }

                    for fragment in &self.fragment_list {
                        if channel
                            .send(WatchStreamEvent::MediaSegment(fragment.clone()))
                            .await
                            .is_err()
                        {
                            // It seems the transcoder has already closed.
                            tracing::warn!("transcoder closed during initialization");
                            return self.request_transcoder(update_channel, global).await;
                        }
                    }

                    self.fragment_list.clear();
                    self.next_transcoder_id = None;
                    self.current_transcoder_id = Some(id);
                    self.current_transcoder = Some(channel);
                }
            }
        }

        true
    }

    async fn on_init_segment(
        &mut self,
        update_channel: &mpsc::Sender<Vec<Update>>,
        global: &Arc<GlobalState>,
        video_settings: &VideoSettings,
        audio_settings: &AudioSettings,
        init_data: Bytes,
    ) -> bool {
        let variants = generate_variants(video_settings, audio_settings, self.api_resp.transcode);

        // We can now at this point decide what we want to do with the stream.
        // What variants should be transcoded, ect...
        if self.api_resp.try_resume {
            // Check if the new variants are the same as the old ones.
            let mut old_map = self
                .api_resp
                .variants
                .iter()
                .map(|v| (v.name.clone(), v))
                .collect::<HashMap<_, _>>();

            for new_variant in &variants {
                if let Some(old_variant) = old_map.remove(&new_variant.name) {
                    let video_same = if let Some(old_video) = &old_variant.video_settings {
                        if let Some(new_video) = &new_variant.video_settings {
                            old_video.codec == new_video.codec
                                && old_video.bitrate == new_video.bitrate
                                && old_video.width == new_video.width
                                && old_video.height == new_video.height
                        } else {
                            false
                        }
                    } else {
                        new_variant.video_settings.is_none()
                    };

                    let audio_same = if let Some(old_audio) = &old_variant.audio_settings {
                        if let Some(new_audio) = &new_variant.audio_settings {
                            old_audio.codec == new_audio.codec
                                && old_audio.bitrate == new_audio.bitrate
                                && old_audio.channels == new_audio.channels
                                && old_audio.sample_rate == new_audio.sample_rate
                        } else {
                            false
                        }
                    } else {
                        new_variant.audio_settings.is_none()
                    };

                    if video_same && audio_same && old_variant.metadata == new_variant.metadata {
                        continue;
                    }
                }

                // If we get here, we need to start a new transcode.
                tracing::info!("new variant detected, starting new transcode");
                self.api_resp.try_resume = false;
                break;
            }

            self.api_resp.try_resume = self.api_resp.try_resume && old_map.is_empty();

            if !self.api_resp.try_resume {
                // Report to API to get a new stream id.
                // This is because the variants have changed and therefore the client player wont be able to resume.
                // We need to get a new stream id so that the player can start a new session.

                let response = match self
                    .api_client
                    .new_live_stream(NewLiveStreamRequest {
                        old_stream_id: self.api_resp.id.to_string(),
                        variants: variants.clone(),
                    })
                    .await
                {
                    Ok(response) => response.into_inner(),
                    Err(e) => {
                        tracing::error!("Failed to report new stream to API: {}", e);
                        return false;
                    }
                };

                let Ok(stream_id) = response.stream_id.parse() else {
                    tracing::error!("invalid stream id from API");
                    return false;
                };

                self.api_resp.id = stream_id;
                self.api_resp.variants = variants;
            }
        } else if let Err(e) = self
            .api_client
            .update_live_stream(UpdateLiveStreamRequest {
                stream_id: self.api_resp.id.to_string(),
                connection_id: self.id.to_string(),
                updates: vec![Update {
                    timestamp: Utc::now().timestamp() as u64,
                    update: Some(update::Update::Variants(
                        update_live_stream_request::Variants {
                            variants: variants.clone(),
                        },
                    )),
                }],
            })
            .await
        {
            tracing::error!("Failed to report new stream to API: {}", e);
            return false;
        } else {
            self.api_resp.variants = variants;
        }

        // At this point now we need to create a new job for a transcoder to pick up and start transcoding.
        global
            .connection_manager
            .register_stream(self.api_resp.id, self.id, self.transcoder_req_tx.clone())
            .await;

        self.initial_segment = Some(init_data);

        if !self.request_transcoder(update_channel, global).await {
            return false;
        }

        // Respond to the rest of the session that we have a stream id and are ready to start streaming.
        self.stream_id_sender.send(self.api_resp.id).is_ok()
    }

    async fn on_data(
        &mut self,
        update_channel: &mpsc::Sender<Vec<Update>>,
        global: &Arc<GlobalState>,
        data: ChannelData,
    ) -> bool {
        if self.bytes_since_keyframe > MAX_BYTES_BETWEEN_KEYFRAMES {
            tracing::error!("keyframe interval exceeded");

            if update_channel
                .try_send(vec![Update {
                    timestamp: Utc::now().timestamp() as u64,
                    update: Some(update::Update::Event(Event {
                        title: "Keyframe Interval Reached".to_string(),
                        level: event::Level::Error as i32,
                        message: "Waited too long without a keyframe, dropping stream".to_string(),
                    })),
                }])
                .is_err()
            {
                tracing::error!("failed to keyframe interval reached");
            }

            return false;
        }

        if (self.total_video_bytes + self.total_audio_bytes + self.total_metadata_bytes)
            >= MAX_BITRATE * BITRATE_UPDATE_INTERVAL / 8
        {
            tracing::error!("bitrate limit reached");

            if update_channel
                .try_send(vec![Update {
                    timestamp: Utc::now().timestamp() as u64,
                    update: Some(update::Update::Event(Event {
                        title: "Bitrate Limit Reached".to_string(),
                        level: event::Level::Error as i32,
                        message: format!(
                            "Reached bitrate limit of {}kbps for stream",
                            (self.total_video_bytes
                                + self.total_audio_bytes
                                + self.total_metadata_bytes)
                                / 1024,
                        ),
                    })),
                }])
                .is_err()
            {
                tracing::error!("failed to send bitrate limit reached event");
            }

            return false;
        }

        match data {
            ChannelData::Video { data, timestamp } => {
                self.total_video_bytes += data.len() as u64;
                self.bytes_since_keyframe += data.len() as u64;

                let data = match FlvTagData::demux(FlvTagType::Video as u8, data) {
                    Ok(data) => data,
                    Err(e) => {
                        tracing::error!(error = %e, "demux error");
                        return false;
                    }
                };

                self.transmuxer.add_tag(FlvTag {
                    timestamp,
                    data,
                    stream_id: 0,
                });
            }
            ChannelData::Audio { data, timestamp } => {
                self.total_audio_bytes += data.len() as u64;
                self.bytes_since_keyframe += data.len() as u64;

                let data = match FlvTagData::demux(FlvTagType::Audio as u8, data) {
                    Ok(data) => data,
                    Err(e) => {
                        tracing::error!(error = %e, "demux error");
                        return false;
                    }
                };

                self.transmuxer.add_tag(FlvTag {
                    timestamp,
                    data,
                    stream_id: 0,
                });
            }
            ChannelData::MetaData { data, timestamp } => {
                self.total_metadata_bytes += data.len() as u64;
                self.bytes_since_keyframe += data.len() as u64;

                let data = match FlvTagData::demux(FlvTagType::ScriptData as u8, data) {
                    Ok(data) => data,
                    Err(e) => {
                        tracing::error!(error = %e, "demux error");
                        return false;
                    }
                };

                self.transmuxer.add_tag(FlvTag {
                    timestamp,
                    data,
                    stream_id: 0,
                });
            }
        }

        // We need to check if the transmuxer has any packets ready to be muxed
        match self.transmuxer.mux() {
            Ok(Some(TransmuxResult::InitSegment {
                video_settings,
                audio_settings,
                data,
            })) => {
                if video_settings.bitrate as u64 + audio_settings.bitrate as u64 >= MAX_BITRATE {
                    tracing::error!("bitrate limit reached");

                    if update_channel
                        .try_send(vec![Update {
                            timestamp: Utc::now().timestamp() as u64,
                            update: Some(update::Update::Event(Event {
                                title: "Bitrate Limit Reached".to_string(),
                                level: event::Level::Error as i32,
                                message: format!(
                                    "Reached bitrate limit of {}kbps for stream",
                                    video_settings.bitrate + audio_settings.bitrate
                                ),
                            })),
                        }])
                        .is_err()
                    {
                        tracing::error!("failed to send bitrate limit reached event");
                    }

                    return false;
                }

                self.on_init_segment(
                    update_channel,
                    global,
                    &video_settings,
                    &audio_settings,
                    data,
                )
                .await
            }
            Ok(Some(TransmuxResult::MediaSegment(segment))) => {
                self.on_media_segment(update_channel, global, segment).await
            }
            Ok(None) => true,
            Err(e) => {
                tracing::error!("error muxing packet: {}", e);
                false
            }
        }
    }

    pub async fn on_media_segment(
        &mut self,
        update_channel: &mpsc::Sender<Vec<Update>>,
        global: &Arc<GlobalState>,
        segment: MediaSegment,
    ) -> bool {
        if segment.keyframe {
            self.bytes_since_keyframe = 0;

            if let Some(transcoder) = self.next_transcoder.take() {
                let Some(uuid) = self.next_transcoder_id.take() else {
                    tracing::error!("next transcoder id is missing");
                    return false;
                };

                if transcoder
                    .send(WatchStreamEvent::MediaSegment(segment.clone()))
                    .await
                    .is_ok()
                {
                    if let Some(current_transcoder) = self.current_transcoder.take() {
                        current_transcoder
                            .send(WatchStreamEvent::ShuttingDown(false))
                            .await
                            .ok();
                    }

                    self.last_transcoder_publish = Instant::now();
                    self.current_transcoder = Some(transcoder);
                    self.current_transcoder_id = Some(uuid);

                    return true;
                }

                if update_channel
                    .try_send(vec![Update {
                        timestamp: Utc::now().timestamp() as u64,
                        update: Some(update::Update::Event(Event {
                            title: "New Transcoder Disconnected".to_string(),
                            level: event::Level::Warning as i32,
                            message: format!(
                                "New Transcoder {} disconnected before sending first fragment",
                                uuid
                            ),
                        })),
                    }])
                    .is_err()
                {
                    tracing::error!("api update channel blocked");
                    return false;
                }

                tracing::error!("new transcoder disconnected before sending first fragment");

                // The next transcoder has disconnected somehow so we need to find a new one.
                if !self.request_transcoder(update_channel, global).await {
                    return false;
                }
            };
        }

        if let Some(transcoder) = &mut self.current_transcoder {
            if transcoder
                .send(WatchStreamEvent::MediaSegment(segment.clone()))
                .await
                .is_ok()
            {
                self.last_transcoder_publish = Instant::now();
                return true;
            }

            tracing::error!("transcoder disconnected while sending fragment");

            let current_id = self.current_transcoder_id.take().unwrap_or_default();

            self.current_transcoder = None;

            if update_channel
                .try_send(vec![Update {
                    timestamp: Utc::now().timestamp() as u64,
                    update: Some(update::Update::Event(Event {
                        title: "Transcoder Disconnected".to_string(),
                        level: event::Level::Warning as i32,
                        message: format!(
                            "Transcoder {} disconnected without graceful shutdown",
                            current_id
                        ),
                    })),
                }])
                .is_err()
            {
                tracing::error!("api update channel blocked");
                return false;
            }

            // This means the current transcoder has disconnected so we need to find a new one.
            if !self.request_transcoder(update_channel, global).await {
                return false;
            }
        }

        if Instant::now() - self.last_transcoder_publish
            >= Duration::from_secs(MAX_TRANSCODER_WAIT_TIME)
        {
            tracing::error!("no transcoder available to publish to");
            return false;
        }

        if segment.keyframe {
            self.fragment_list.clear();
            self.fragment_list.push(segment);
        } else if self
            .fragment_list
            .first()
            .map(|f| f.keyframe)
            .unwrap_or_default()
        {
            self.fragment_list.push(segment);
        }

        true
    }

    fn on_bitrate_update(&mut self, update_channel: &mpsc::Sender<Vec<Update>>) -> bool {
        let video_bitrate = (self.total_video_bytes * 8) / BITRATE_UPDATE_INTERVAL;
        let audio_bitrate = (self.total_audio_bytes * 8) / BITRATE_UPDATE_INTERVAL;
        let metadata_bitrate = (self.total_metadata_bytes * 8) / BITRATE_UPDATE_INTERVAL;

        self.total_video_bytes = 0;
        self.total_audio_bytes = 0;
        self.total_metadata_bytes = 0;

        // We need to make sure that the update future is still running
        if update_channel
            .try_send(vec![Update {
                timestamp: Utc::now().timestamp() as u64,
                update: Some(update::Update::Bitrate(Bitrate {
                    video_bitrate,
                    audio_bitrate,
                    metadata_bitrate,
                })),
            }])
            .is_err()
        {
            tracing::error!("api update channel blocked");
            return false;
        }

        true
    }
}
