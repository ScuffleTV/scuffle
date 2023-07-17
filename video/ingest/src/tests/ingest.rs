use std::path::{Path, PathBuf};
use std::pin::{pin, Pin};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use async_stream::stream;
use async_trait::async_trait;
use common::config::TlsConfig;
use common::prelude::FutureTimeout;
use futures::StreamExt;
use lapin::options::QueueDeclareOptions;
use prost::Message;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::select;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tonic::{Request, Response, Status};
use transmuxer::MediaType;
use uuid::Uuid;

use crate::config::{ApiConfig, AppConfig, RtmpConfig, TranscoderConfig};
use crate::connection_manager::{GrpcRequest, WatchStreamEvent};
use crate::global;
use crate::pb::scuffle::backend::update_live_stream_request::event::Level;
use crate::pb::scuffle::backend::{
    api_server, update_live_stream_request, AuthenticateLiveStreamRequest,
    AuthenticateLiveStreamResponse, NewLiveStreamRequest, NewLiveStreamResponse, StreamReadyState,
    UpdateLiveStreamRequest, UpdateLiveStreamResponse,
};
use crate::pb::scuffle::events::{transcoder_message, TranscoderMessage};
use crate::pb::scuffle::types::{stream_state, StreamState};
use crate::tests::global::mock_global_state;

#[derive(Debug)]
enum IncomingRequest {
    Authenticate(
        (
            AuthenticateLiveStreamRequest,
            oneshot::Sender<Result<AuthenticateLiveStreamResponse>>,
        ),
    ),
    Update(
        (
            UpdateLiveStreamRequest,
            oneshot::Sender<Result<UpdateLiveStreamResponse>>,
        ),
    ),
    New(
        (
            NewLiveStreamRequest,
            oneshot::Sender<Result<NewLiveStreamResponse>>,
        ),
    ),
}

struct ApiServer(mpsc::Sender<IncomingRequest>);

fn new_api_server(port: u16) -> mpsc::Receiver<IncomingRequest> {
    let (tx, rx) = mpsc::channel(1);
    let service = api_server::ApiServer::new(ApiServer(tx));

    tokio::spawn(
        tonic::transport::Server::builder()
            .add_service(service)
            .serve(format!("0.0.0.0:{}", port).parse().unwrap()),
    );

    rx
}

type Result<T> = std::result::Result<T, Status>;

#[async_trait]
impl crate::pb::scuffle::backend::api_server::Api for ApiServer {
    async fn authenticate_live_stream(
        &self,
        request: Request<AuthenticateLiveStreamRequest>,
    ) -> Result<Response<AuthenticateLiveStreamResponse>> {
        let (send, recv) = oneshot::channel();
        self.0
            .send(IncomingRequest::Authenticate((request.into_inner(), send)))
            .await
            .unwrap();
        Ok(Response::new(recv.await.unwrap()?))
    }

    async fn update_live_stream(
        &self,
        request: Request<UpdateLiveStreamRequest>,
    ) -> Result<Response<UpdateLiveStreamResponse>> {
        let (send, recv) = oneshot::channel();
        self.0
            .send(IncomingRequest::Update((request.into_inner(), send)))
            .await
            .unwrap();
        Ok(Response::new(recv.await.unwrap()?))
    }

    async fn new_live_stream(
        &self,
        request: Request<NewLiveStreamRequest>,
    ) -> Result<Response<NewLiveStreamResponse>> {
        let (send, recv) = oneshot::channel();
        self.0
            .send(IncomingRequest::New((request.into_inner(), send)))
            .await
            .unwrap();
        Ok(Response::new(recv.await.unwrap()?))
    }
}

fn stream_with_ffmpeg(rtmp_port: u16, file: &str) -> tokio::process::Child {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets");

    Command::new("ffmpeg")
        .args([
            "-re",
            "-i",
            dir.join(file).to_str().expect("failed to get path"),
            "-c",
            "copy",
            "-f",
            "flv",
            &format!("rtmp://127.0.0.1:{}/live/stream-key", rtmp_port),
        ])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .expect("failed to execute ffmpeg")
}

fn stream_with_ffmpeg_tls(rtmp_port: u16, file: &str, tls_dir: &Path) -> tokio::process::Child {
    let video_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets");

    Command::new("ffmpeg")
        .args([
            "-re",
            "-i",
            video_dir.join(file).to_str().expect("failed to get path"),
            "-c",
            "copy",
            "-tls_verify",
            "1",
            "-ca_file",
            tls_dir.join("ca.crt").to_str().unwrap(),
            "-cert_file",
            tls_dir.join("client.crt").to_str().unwrap(),
            "-key_file",
            tls_dir.join("client.key").to_str().unwrap(),
            "-f",
            "flv",
            &format!("rtmps://localhost:{}/live/stream-key", rtmp_port),
        ])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .expect("failed to execute ffmpeg")
}

fn spawn_ffprobe() -> tokio::process::Child {
    Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-fpsprobesize")
        .arg("20000")
        .arg("-show_format")
        .arg("-show_streams")
        .arg("-print_format")
        .arg("json")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap()
}

struct Watcher {
    pub rx: tokio::sync::mpsc::Receiver<WatchStreamEvent>,
}

impl Watcher {
    async fn new(state: &TestState, stream_id: Uuid, request_id: Uuid) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        assert!(
            state
                .global
                .connection_manager
                .submit_request(
                    stream_id,
                    GrpcRequest::WatchStream {
                        id: request_id,
                        channel: tx,
                    }
                )
                .await
        );
        Self { rx }
    }

    async fn recv(&mut self) -> WatchStreamEvent {
        tokio::time::timeout(Duration::from_secs(2), self.rx.recv())
            .await
            .expect("failed to receive event")
            .expect("failed to receive event")
    }
}

struct TestState {
    pub rtmp_port: u16,
    pub global: Arc<global::GlobalState>,
    pub handler: common::context::Handler,
    pub api_rx: mpsc::Receiver<IncomingRequest>,
    pub transcoder_stream: Pin<Box<dyn futures::Stream<Item = TranscoderMessage>>>,
    pub ingest_handle: JoinHandle<anyhow::Result<()>>,
}

impl TestState {
    async fn setup() -> Self {
        Self::setup_new(None).await
    }

    async fn setup_with_tls(tls_dir: &Path) -> Self {
        Self::setup_new(Some(TlsConfig {
            cert: tls_dir.join("server.crt").to_str().unwrap().to_string(),
            ca_cert: tls_dir.join("ca.crt").to_str().unwrap().to_string(),
            key: tls_dir.join("server.key").to_str().unwrap().to_string(),
            domain: Some("localhost".to_string()),
        }))
        .await
    }

    async fn setup_new(tls: Option<TlsConfig>) -> Self {
        let api_port = portpicker::pick_unused_port().unwrap();
        let rtmp_port = portpicker::pick_unused_port().unwrap();

        let api_rx = new_api_server(api_port);

        let (global, handler) = mock_global_state(AppConfig {
            api: ApiConfig {
                addresses: vec![format!("http://localhost:{}", api_port)],
                resolve_interval: 1,
                tls: None,
            },
            rtmp: RtmpConfig {
                bind_address: format!("0.0.0.0:{}", rtmp_port).parse().unwrap(),
                tls,
            },
            transcoder: TranscoderConfig {
                events_subject: Uuid::new_v4().to_string(),
            },
            ..Default::default()
        })
        .await;

        global
            .rmq
            .aquire()
            .await
            .unwrap()
            .queue_declare(
                &global.config.transcoder.events_subject.clone(),
                QueueDeclareOptions {
                    auto_delete: true,
                    durable: false,
                    ..Default::default()
                },
                Default::default(),
            )
            .await
            .unwrap();

        let ingest_handle = tokio::spawn(crate::ingest::run(global.clone()));

        let stream = {
            let global = global.clone();
            stream!({
                let mut stream = pin!(global.rmq.basic_consume(
                    global.config.transcoder.events_subject.clone(),
                    "",
                    Default::default(),
                    Default::default()
                ));
                loop {
                    select! {
                        message = stream.next() => {
                            let message = message.unwrap().unwrap();
                            yield TranscoderMessage::decode(message.data.as_slice()).unwrap();
                        }
                        _ = global.ctx.done() => {
                            break;
                        }
                    }
                }
            })
        };

        Self {
            rtmp_port,
            global,
            handler,
            api_rx,
            transcoder_stream: Box::pin(stream),
            ingest_handle,
        }
    }

    async fn transcoder_message(&mut self) -> TranscoderMessage {
        tokio::time::timeout(Duration::from_secs(2), self.transcoder_stream.next())
            .await
            .expect("failed to receive event")
            .expect("failed to receive event")
    }

    async fn api_recv(&mut self) -> IncomingRequest {
        tokio::time::timeout(Duration::from_secs(2), self.api_rx.recv())
            .await
            .expect("failed to receive event")
            .expect("failed to receive event")
    }

    fn finish(self) -> impl futures::Future<Output = ()> {
        let handler = self.handler;
        let ingest_handle = self.ingest_handle;
        async move {
            handler.cancel().await;
            assert!(ingest_handle.is_finished());
        }
    }

    async fn api_assert_authenticate(&mut self, response: Result<AuthenticateLiveStreamResponse>) {
        match self.api_recv().await {
            IncomingRequest::Authenticate((request, send)) => {
                assert_eq!(request.stream_key, "stream-key");
                assert_eq!(request.app_name, "live");
                assert!(!request.connection_id.is_empty());
                assert!(!request.ingest_address.is_empty());
                send.send(response).unwrap();
            }
            _ => panic!("unexpected event"),
        }
    }

    async fn api_assert_authenticate_ok(&mut self, record: bool, transcode: bool) -> Uuid {
        let stream_id = Uuid::new_v4();
        self.api_assert_authenticate(Ok(AuthenticateLiveStreamResponse {
            stream_id: stream_id.to_string(),
            record,
            transcode,
            state: None,
        }))
        .await;
        stream_id
    }
}

#[tokio::test]
async fn test_ingest_stream() {
    let mut state = TestState::setup().await;
    let mut ffmpeg = stream_with_ffmpeg(state.rtmp_port, "avc_aac_keyframes.mp4");

    let stream_id = state.api_assert_authenticate_ok(false, false).await;

    let stream_state;
    match state.api_recv().await {
        IncomingRequest::Update((request, send)) => {
            assert_eq!(request.stream_id, stream_id.to_string());
            match &request.updates[0].update {
                Some(
                    crate::pb::scuffle::backend::update_live_stream_request::update::Update::State(
                        v,
                    ),
                ) => {
                    assert_eq!(v.transcodes.len(), 2); // We are not transcoding so this is source and audio only
                    assert_eq!(v.variants.len(), 2); // We are not transcoding so this is source and audio only

                    let source_variant = v.variants.iter().find(|v| v.name == "source").unwrap();
                    assert_eq!(source_variant.group, "aac");
                    assert_eq!(source_variant.transcode_ids.len(), 2);

                    let audio_only_variant =
                        v.variants.iter().find(|v| v.name == "audio-only").unwrap();
                    assert_eq!(audio_only_variant.group, "aac");
                    assert_eq!(audio_only_variant.transcode_ids.len(), 1);

                    let audio_transcode_state = v
                        .transcodes
                        .iter()
                        .find(|s| s.id == audio_only_variant.transcode_ids[0])
                        .unwrap();

                    assert_eq!(audio_transcode_state.bitrate, 128 * 1024);
                    assert_eq!(audio_transcode_state.codec, "mp4a.40.2");
                    assert_eq!(
                        audio_transcode_state.settings,
                        Some(stream_state::transcode::Settings::Audio(
                            stream_state::transcode::AudioSettings {
                                channels: 2,
                                sample_rate: 48000,
                            }
                        ))
                    );
                    assert!(!audio_transcode_state.copy);

                    let source_transcode_state = v
                        .transcodes
                        .iter()
                        .find(|s| s.id == source_variant.transcode_ids[0])
                        .unwrap();

                    assert_eq!(source_transcode_state.codec, "avc1.64001f");
                    assert_eq!(source_transcode_state.bitrate, 1276158);
                    assert_eq!(
                        source_transcode_state.settings,
                        Some(stream_state::transcode::Settings::Video(
                            stream_state::transcode::VideoSettings {
                                width: 468,
                                height: 864,
                                framerate: 30,
                            }
                        ))
                    );
                    assert!(source_transcode_state.copy);

                    stream_state = Some(v.clone());

                    send.send(Ok(UpdateLiveStreamResponse {})).unwrap();
                }
                _ => panic!("unexpected update"),
            }
        }
        _ => panic!("unexpected event"),
    }

    match state.api_recv().await {
        IncomingRequest::Update((update, response)) => {
            assert_eq!(update.stream_id, stream_id.to_string());
            assert_eq!(update.updates.len(), 1);

            let update = &update.updates[0];
            assert!(update.timestamp > 0);

            match &update.update {
                Some(update_live_stream_request::update::Update::Event(event)) => {
                    assert_eq!(event.title, "Requested Transcoder");
                    assert_eq!(
                        event.message,
                        "Requested a transcoder to be assigned to this stream"
                    );
                    assert_eq!(event.level, Level::Info as i32)
                }
                u => {
                    panic!("unexpected update: {:?}", u);
                }
            }

            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    let msg = state.transcoder_message().await;
    assert!(!msg.id.is_empty());
    assert!(msg.timestamp > 0);
    let data = match msg.data {
        Some(transcoder_message::Data::NewStream(data)) => data,
        _ => panic!("unexpected message"),
    };

    assert!(!data.request_id.is_empty());
    assert_eq!(data.stream_id, stream_id.to_string());
    assert_eq!(data.state, stream_state);

    // We should now be able to join the stream
    let stream_id = data.stream_id.parse().unwrap();
    let request_id = data.request_id.parse().unwrap();
    let mut watcher = Watcher::new(&state, stream_id, request_id).await;

    match watcher.recv().await {
        WatchStreamEvent::InitSegment(data) => assert!(!data.is_empty()),
        _ => panic!("unexpected event"),
    }

    match watcher.recv().await {
        WatchStreamEvent::MediaSegment(ms) => {
            assert!(!ms.data.is_empty());
            assert!(ms.keyframe);
            assert_eq!(ms.ty, MediaType::Video);
            assert_eq!(ms.timestamp, 0);
        }
        _ => panic!("unexpected event"),
    }

    state
        .global
        .connection_manager
        .submit_request(
            stream_id,
            GrpcRequest::TranscoderShuttingDown { id: request_id },
        )
        .await;

    match state.api_recv().await {
        IncomingRequest::Update((update, response)) => {
            assert_eq!(update.stream_id, stream_id.to_string());
            assert_eq!(update.updates.len(), 1);

            let update = &update.updates[0];
            assert!(update.timestamp > 0);

            match &update.update {
                Some(update_live_stream_request::update::Update::Event(event)) => {
                    assert_eq!(event.title, "Requested Transcoder");
                    assert_eq!(
                        event.message,
                        "Requested a transcoder to be assigned to this stream"
                    );
                    assert_eq!(event.level, Level::Info as i32)
                }
                u => {
                    panic!("unexpected update: {:?}", u);
                }
            }

            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    // It should now create a new transcoder to handle the stream
    let msg = state.transcoder_message().await;
    assert!(!msg.id.is_empty());
    assert!(msg.timestamp > 0);
    let data = match msg.data {
        Some(transcoder_message::Data::NewStream(data)) => data,
        _ => panic!("unexpected message"),
    };

    assert!(!data.request_id.is_empty());
    assert_eq!(data.stream_id, stream_id.to_string());
    assert_eq!(data.state, stream_state);

    // We should now be able to join the stream
    let stream_id = data.stream_id.parse().unwrap();
    let request_id = data.request_id.parse().unwrap();
    let mut new_watcher = Watcher::new(&state, stream_id, request_id).await;

    let mut previous_audio_ts = 0;
    let mut previous_video_ts = 0;
    let mut got_shutting_down = false;
    while let Some(msg) = watcher.rx.recv().await {
        match msg {
            WatchStreamEvent::MediaSegment(ms) => {
                assert!(!ms.data.is_empty());
                assert!(!ms.keyframe);
                match ms.ty {
                    MediaType::Audio => {
                        assert!(ms.timestamp >= previous_audio_ts);
                        previous_audio_ts = ms.timestamp;
                    }
                    MediaType::Video => {
                        assert!(ms.timestamp >= previous_video_ts);
                        previous_video_ts = ms.timestamp;
                    }
                }
            }
            WatchStreamEvent::ShuttingDown(false) => {
                got_shutting_down = true;
                break;
            }
            _ => panic!("unexpected event"),
        }
    }

    assert!(got_shutting_down);

    match new_watcher.recv().await {
        WatchStreamEvent::InitSegment(data) => assert!(!data.is_empty()),
        _ => panic!("unexpected event"),
    }

    match new_watcher.recv().await {
        WatchStreamEvent::MediaSegment(ms) => {
            assert!(!ms.data.is_empty());
            assert!(ms.keyframe);
            assert_eq!(ms.timestamp, 1000);
            assert_eq!(ms.ty, MediaType::Video);
            previous_video_ts = 1000;
        }
        _ => panic!("unexpected event"),
    }

    while let Ok(msg) = new_watcher.rx.try_recv() {
        match msg {
            WatchStreamEvent::MediaSegment(ms) => {
                assert!(!ms.data.is_empty());
                match ms.ty {
                    MediaType::Audio => {
                        assert!(ms.timestamp >= previous_audio_ts);
                        previous_audio_ts = ms.timestamp;
                    }
                    MediaType::Video => {
                        assert!(ms.timestamp >= previous_video_ts);
                        previous_video_ts = ms.timestamp;
                    }
                }
            }
            _ => panic!("unexpected event"),
        }
    }

    // Assert that no messages with keyframes made it to the old channel

    ffmpeg.kill().await.unwrap();

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Assert that the stream is removed
    assert!(
        !state
            .global
            .connection_manager
            .submit_request(stream_id, GrpcRequest::TranscoderStarted { id: request_id })
            .await
    );

    // Assert that the stream is removed
    match state.api_recv().await {
        IncomingRequest::Update((update, response)) => {
            assert_eq!(update.stream_id, stream_id.to_string());
            assert_eq!(update.updates.len(), 1);

            let update = &update.updates[0];
            assert!(update.timestamp > 0);

            match &update.update {
                Some(update_live_stream_request::update::Update::ReadyState(state)) => {
                    assert_eq!(*state, StreamReadyState::StoppedResumable as i32);
                }
                u => {
                    panic!("unexpected update: {:?}", u);
                }
            }

            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    tracing::info!("waiting for transcoder to exit");

    state.finish().await;
}

#[tokio::test]
async fn test_ingest_stream_transcoder_disconnect() {
    let mut state = TestState::setup().await;
    let mut ffmpeg = stream_with_ffmpeg(state.rtmp_port, "avc_aac_keyframes.mp4");

    let stream_id = state.api_assert_authenticate_ok(false, true).await;

    let stream_state;
    match state.api_recv().await {
        IncomingRequest::Update((request, send)) => {
            assert_eq!(request.stream_id, stream_id.to_string());
            match &request.updates[0].update {
                Some(
                    crate::pb::scuffle::backend::update_live_stream_request::update::Update::State(
                        v,
                    ),
                ) => {
                    assert_eq!(v.transcodes.len(), 4); // We are not transcoding so this is source and audio only
                    assert_eq!(v.variants.len(), 6); // We are not transcoding so this is source and audio only

                    let audio_only_aac = v
                        .variants
                        .iter()
                        .find(|v| v.name == "audio-only" && v.group == "aac")
                        .unwrap();
                    assert_eq!(audio_only_aac.transcode_ids.len(), 1);

                    let audio_only_opus = v
                        .variants
                        .iter()
                        .find(|v| v.name == "audio-only" && v.group == "opus")
                        .unwrap();
                    assert_eq!(audio_only_opus.transcode_ids.len(), 1);

                    let source_aac = v
                        .variants
                        .iter()
                        .find(|v| v.name == "source" && v.group == "aac")
                        .unwrap();
                    assert_eq!(source_aac.transcode_ids.len(), 2);

                    let source_opus = v
                        .variants
                        .iter()
                        .find(|v| v.name == "source" && v.group == "opus")
                        .unwrap();
                    assert_eq!(source_opus.transcode_ids.len(), 2);

                    let _360p_aac = v
                        .variants
                        .iter()
                        .find(|v| v.name == "360p" && v.group == "aac")
                        .unwrap();
                    assert_eq!(_360p_aac.transcode_ids.len(), 2);

                    let _360p_opus = v
                        .variants
                        .iter()
                        .find(|v| v.name == "360p" && v.group == "opus")
                        .unwrap();
                    assert_eq!(_360p_opus.transcode_ids.len(), 2);

                    let audio_aac_transcode_state = v
                        .transcodes
                        .iter()
                        .find(|s| s.id == audio_only_aac.transcode_ids[0])
                        .unwrap();
                    assert!(!audio_aac_transcode_state.copy);
                    assert_eq!(audio_aac_transcode_state.codec, "mp4a.40.2");
                    assert_eq!(audio_aac_transcode_state.bitrate, 128 * 1024);
                    assert_eq!(
                        audio_aac_transcode_state.settings,
                        Some(stream_state::transcode::Settings::Audio(
                            stream_state::transcode::AudioSettings {
                                channels: 2,
                                sample_rate: 48000,
                            }
                        ))
                    );

                    let audio_opus_transcode_state = v
                        .transcodes
                        .iter()
                        .find(|s| s.id == audio_only_opus.transcode_ids[0])
                        .unwrap();
                    assert!(!audio_opus_transcode_state.copy);
                    assert_eq!(audio_opus_transcode_state.codec, "opus");
                    assert_eq!(audio_opus_transcode_state.bitrate, 96 * 1024);
                    assert_eq!(
                        audio_opus_transcode_state.settings,
                        Some(stream_state::transcode::Settings::Audio(
                            stream_state::transcode::AudioSettings {
                                channels: 2,
                                sample_rate: 48000,
                            }
                        ))
                    );

                    let source_video_transcode_state = v
                        .transcodes
                        .iter()
                        .find(|s| s.id == source_aac.transcode_ids[0])
                        .unwrap();
                    assert!(source_video_transcode_state.copy);
                    assert_eq!(source_video_transcode_state.codec, "avc1.64001f");
                    assert_eq!(source_video_transcode_state.bitrate, 1276158);
                    assert_eq!(
                        source_video_transcode_state.settings,
                        Some(stream_state::transcode::Settings::Video(
                            stream_state::transcode::VideoSettings {
                                width: 468,
                                height: 864,
                                framerate: 30,
                            }
                        ))
                    );

                    assert_eq!(source_aac.transcode_ids[0], source_opus.transcode_ids[0]);
                    assert_eq!(source_aac.transcode_ids[1], audio_aac_transcode_state.id);
                    assert_eq!(source_opus.transcode_ids[1], audio_opus_transcode_state.id);

                    let _360p_video_transcode_state = v
                        .transcodes
                        .iter()
                        .find(|s| s.id == _360p_aac.transcode_ids[0])
                        .unwrap();
                    assert!(!_360p_video_transcode_state.copy);
                    assert_eq!(_360p_video_transcode_state.codec, "avc1.640033");
                    assert_eq!(_360p_video_transcode_state.bitrate, 1024000);
                    assert_eq!(
                        _360p_video_transcode_state.settings,
                        Some(stream_state::transcode::Settings::Video(
                            stream_state::transcode::VideoSettings {
                                width: 360,
                                height: 665,
                                framerate: 30,
                            }
                        ))
                    );

                    assert_eq!(_360p_aac.transcode_ids[0], _360p_opus.transcode_ids[0]);
                    assert_eq!(_360p_aac.transcode_ids[1], audio_aac_transcode_state.id);
                    assert_eq!(_360p_opus.transcode_ids[1], audio_opus_transcode_state.id);

                    stream_state = Some(v.clone());

                    send.send(Ok(UpdateLiveStreamResponse {})).unwrap();
                }
                _ => panic!("unexpected update"),
            }
        }
        _ => panic!("unexpected event"),
    }

    match state.api_recv().await {
        IncomingRequest::Update((update, response)) => {
            assert_eq!(update.stream_id, stream_id.to_string());
            assert_eq!(update.updates.len(), 1);

            let update = &update.updates[0];
            assert!(update.timestamp > 0);

            match &update.update {
                Some(update_live_stream_request::update::Update::Event(event)) => {
                    assert_eq!(event.title, "Requested Transcoder");
                    assert_eq!(
                        event.message,
                        "Requested a transcoder to be assigned to this stream"
                    );
                    assert_eq!(event.level, Level::Info as i32)
                }
                u => {
                    panic!("unexpected update: {:?}", u);
                }
            }

            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    let msg = state.transcoder_message().await;
    assert!(!msg.id.is_empty());
    assert!(msg.timestamp > 0);
    let data = match msg.data {
        Some(transcoder_message::Data::NewStream(data)) => data,
        _ => panic!("unexpected message"),
    };

    assert!(!data.request_id.is_empty());
    assert_eq!(data.stream_id, stream_id.to_string());
    assert_eq!(data.state, stream_state);

    // We should now be able to join the stream
    let stream_id = data.stream_id.parse().unwrap();
    let request_id = data.request_id.parse().unwrap();
    let mut watcher = Watcher::new(&state, stream_id, request_id).await;

    match watcher.recv().await {
        WatchStreamEvent::InitSegment(data) => assert!(!data.is_empty()),
        _ => panic!("unexpected event"),
    }

    match watcher.recv().await {
        WatchStreamEvent::MediaSegment(ms) => {
            assert!(!ms.data.is_empty());
            assert!(ms.keyframe);
        }
        _ => panic!("unexpected event"),
    }

    // Force disconnect the transcoder
    drop(watcher);

    match state.api_recv().await {
        IncomingRequest::Update((update, response)) => {
            assert_eq!(update.stream_id, stream_id.to_string());
            assert_eq!(update.updates.len(), 1);

            let update = &update.updates[0];
            assert!(update.timestamp > 0);

            match &update.update {
                Some(update_live_stream_request::update::Update::Event(event)) => {
                    assert_eq!(event.title, "Transcoder Disconnected");
                    assert_eq!(event.level, Level::Warning as i32)
                }
                u => {
                    panic!("unexpected update: {:?}", u);
                }
            }

            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    match state.api_recv().await {
        IncomingRequest::Update((update, response)) => {
            assert_eq!(update.stream_id, stream_id.to_string());
            assert_eq!(update.updates.len(), 1);

            let update = &update.updates[0];
            assert!(update.timestamp > 0);

            match &update.update {
                Some(update_live_stream_request::update::Update::Event(event)) => {
                    assert_eq!(event.title, "Requested Transcoder");
                    assert_eq!(
                        event.message,
                        "Requested a transcoder to be assigned to this stream"
                    );
                    assert_eq!(event.level, Level::Info as i32)
                }
                u => {
                    panic!("unexpected update: {:?}", u);
                }
            }

            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    // It should now create a new transcoder to handle the stream
    let msg = state.transcoder_message().await;
    assert!(!msg.id.is_empty());
    assert!(msg.timestamp > 0);
    let data = match msg.data {
        Some(transcoder_message::Data::NewStream(data)) => data,
        _ => panic!("unexpected message"),
    };

    assert!(!data.request_id.is_empty());
    assert_eq!(data.stream_id, stream_id.to_string());
    assert_eq!(data.state, stream_state);

    // We should now be able to join the stream
    let stream_id = data.stream_id.parse().unwrap();
    let request_id = data.request_id.parse().unwrap();
    let mut watcher = Watcher::new(&state, stream_id, request_id).await;

    match watcher.recv().await {
        WatchStreamEvent::InitSegment(data) => assert!(!data.is_empty()),
        _ => panic!("unexpected event"),
    }

    match watcher.recv().await {
        WatchStreamEvent::MediaSegment(ms) => {
            assert!(!ms.data.is_empty());
            assert!(ms.keyframe);
        }
        _ => panic!("unexpected event"),
    }

    ffmpeg.kill().await.unwrap();

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Assert that the stream is removed
    assert!(
        !state
            .global
            .connection_manager
            .submit_request(stream_id, GrpcRequest::TranscoderStarted { id: request_id })
            .await
    );

    // Assert that the stream is removed
    match state.api_recv().await {
        IncomingRequest::Update((update, response)) => {
            assert_eq!(update.stream_id, stream_id.to_string());
            assert_eq!(update.updates.len(), 1);

            let update = &update.updates[0];
            assert!(update.timestamp > 0);

            match &update.update {
                Some(update_live_stream_request::update::Update::ReadyState(state)) => {
                    assert_eq!(*state, StreamReadyState::StoppedResumable as i32);
                }
                u => {
                    panic!("unexpected update: {:?}", u);
                }
            }

            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    state.finish().await;
}

#[tokio::test]
async fn test_ingest_stream_shutdown() {
    let mut state = TestState::setup().await;
    let mut ffmpeg = stream_with_ffmpeg(state.rtmp_port, "avc_aac_keyframes.mp4");

    let stream_id = state.api_assert_authenticate_ok(false, false).await;

    match state.api_recv().await {
        IncomingRequest::Update((request, send)) => {
            assert_eq!(request.stream_id, stream_id.to_string());
            match &request.updates[0].update {
                Some(
                    crate::pb::scuffle::backend::update_live_stream_request::update::Update::State(
                        v,
                    ),
                ) => {
                    assert_eq!(v.variants.len(), 2);
                    assert_eq!(v.transcodes.len(), 2);
                    send.send(Ok(UpdateLiveStreamResponse {})).unwrap();
                }
                _ => panic!("unexpected update"),
            }
        }
        _ => panic!("unexpected event"),
    }

    match state.api_recv().await {
        IncomingRequest::Update((update, response)) => {
            assert_eq!(update.stream_id, stream_id.to_string());
            assert_eq!(update.updates.len(), 1);

            let update = &update.updates[0];
            assert!(update.timestamp > 0);

            match &update.update {
                Some(update_live_stream_request::update::Update::Event(event)) => {
                    assert_eq!(event.title, "Requested Transcoder");
                    assert_eq!(
                        event.message,
                        "Requested a transcoder to be assigned to this stream"
                    );
                    assert_eq!(event.level, Level::Info as i32)
                }
                u => {
                    panic!("unexpected update: {:?}", u);
                }
            }

            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    assert!(
        state
            .global
            .connection_manager
            .submit_request(stream_id, GrpcRequest::ShutdownStream)
            .await
    );

    tracing::info!("waiting for transcoder to exit");

    assert!(ffmpeg.wait().timeout(Duration::from_secs(1)).await.is_ok());

    state.finish().await;
}

#[tokio::test]
async fn test_ingest_stream_transcoder_full() {
    let mut state = TestState::setup().await;
    let mut ffmpeg = stream_with_ffmpeg(state.rtmp_port, "avc_aac_large.mp4");

    let stream_id = state.api_assert_authenticate_ok(false, true).await;

    let stream_state;
    match state.api_recv().await {
        IncomingRequest::Update((request, send)) => {
            assert_eq!(request.stream_id, stream_id.to_string());
            match &request.updates[0].update {
                Some(
                    crate::pb::scuffle::backend::update_live_stream_request::update::Update::State(
                        v,
                    ),
                ) => {
                    let aac_audio_only = v
                        .variants
                        .iter()
                        .find(|v| v.name == "audio-only" && v.group == "aac")
                        .unwrap();
                    assert_eq!(aac_audio_only.transcode_ids.len(), 1);

                    let opus_audio_only = v
                        .variants
                        .iter()
                        .find(|v| v.name == "audio-only" && v.group == "opus")
                        .unwrap();
                    assert_eq!(opus_audio_only.transcode_ids.len(), 1);

                    let aac_source = v
                        .variants
                        .iter()
                        .find(|v| v.name == "source" && v.group == "aac")
                        .unwrap();
                    assert_eq!(aac_source.transcode_ids.len(), 2);

                    let opus_source = v
                        .variants
                        .iter()
                        .find(|v| v.name == "source" && v.group == "opus")
                        .unwrap();
                    assert_eq!(opus_source.transcode_ids.len(), 2);

                    let aac_720p = v
                        .variants
                        .iter()
                        .find(|v| v.name == "720p" && v.group == "aac")
                        .unwrap();
                    assert_eq!(aac_720p.transcode_ids.len(), 2);

                    let opus_720p = v
                        .variants
                        .iter()
                        .find(|v| v.name == "720p" && v.group == "opus")
                        .unwrap();
                    assert_eq!(opus_720p.transcode_ids.len(), 2);

                    let aac_480p = v
                        .variants
                        .iter()
                        .find(|v| v.name == "480p" && v.group == "aac")
                        .unwrap();
                    assert_eq!(aac_480p.transcode_ids.len(), 2);

                    let opus_480p = v
                        .variants
                        .iter()
                        .find(|v| v.name == "480p" && v.group == "opus")
                        .unwrap();
                    assert_eq!(opus_480p.transcode_ids.len(), 2);

                    let aac_360p = v
                        .variants
                        .iter()
                        .find(|v| v.name == "360p" && v.group == "aac")
                        .unwrap();
                    assert_eq!(aac_360p.transcode_ids.len(), 2);

                    let opus_360p = v
                        .variants
                        .iter()
                        .find(|v| v.name == "360p" && v.group == "opus")
                        .unwrap();
                    assert_eq!(opus_360p.transcode_ids.len(), 2);

                    let aac_transcode_state = v
                        .transcodes
                        .iter()
                        .find(|s| s.id == aac_audio_only.transcode_ids[0])
                        .unwrap();
                    assert_eq!(aac_transcode_state.codec, "mp4a.40.2".to_string());
                    assert_eq!(aac_transcode_state.bitrate, 128 * 1024);
                    assert!(!aac_transcode_state.copy);
                    assert_eq!(
                        aac_transcode_state.settings,
                        Some(stream_state::transcode::Settings::Audio(
                            stream_state::transcode::AudioSettings {
                                channels: 2,
                                sample_rate: 48_000,
                            }
                        ))
                    );

                    let opus_transcode_state = v
                        .transcodes
                        .iter()
                        .find(|s| s.id == opus_audio_only.transcode_ids[0])
                        .unwrap();
                    assert_eq!(opus_transcode_state.codec, "opus".to_string());
                    assert_eq!(opus_transcode_state.bitrate, 96 * 1024);
                    assert!(!opus_transcode_state.copy);
                    assert_eq!(
                        opus_transcode_state.settings,
                        Some(stream_state::transcode::Settings::Audio(
                            stream_state::transcode::AudioSettings {
                                channels: 2,
                                sample_rate: 48_000,
                            }
                        ))
                    );

                    // Now for the video source
                    let source_video_transcode_state = v
                        .transcodes
                        .iter()
                        .find(|s| s.id == aac_source.transcode_ids[0])
                        .unwrap();
                    assert_eq!(
                        source_video_transcode_state.codec,
                        "avc1.640034".to_string()
                    );
                    assert_eq!(source_video_transcode_state.bitrate, 1740285);
                    assert!(source_video_transcode_state.copy);
                    assert_eq!(
                        source_video_transcode_state.settings,
                        Some(stream_state::transcode::Settings::Video(
                            stream_state::transcode::VideoSettings {
                                framerate: 60,
                                height: 2160,
                                width: 3840,
                            }
                        ))
                    );

                    assert_eq!(aac_source.transcode_ids[0], opus_source.transcode_ids[0]);
                    assert_eq!(aac_source.transcode_ids[1], aac_transcode_state.id);
                    assert_eq!(opus_source.transcode_ids[1], opus_transcode_state.id);

                    let _720p_video_transcode_state = v
                        .transcodes
                        .iter()
                        .find(|s| s.id == aac_720p.transcode_ids[0])
                        .unwrap();
                    assert_eq!(_720p_video_transcode_state.codec, "avc1.640033".to_string());
                    assert_eq!(_720p_video_transcode_state.bitrate, 4000 * 1024);
                    assert!(!_720p_video_transcode_state.copy);
                    assert_eq!(
                        _720p_video_transcode_state.settings,
                        Some(stream_state::transcode::Settings::Video(
                            stream_state::transcode::VideoSettings {
                                framerate: 60,
                                height: 720,
                                width: 1280,
                            }
                        ))
                    );

                    assert_eq!(aac_720p.transcode_ids[0], opus_720p.transcode_ids[0]);
                    assert_eq!(aac_720p.transcode_ids[1], aac_transcode_state.id);
                    assert_eq!(opus_720p.transcode_ids[1], opus_transcode_state.id);

                    let _480p_video_transcode_state = v
                        .transcodes
                        .iter()
                        .find(|s| s.id == aac_480p.transcode_ids[0])
                        .unwrap();
                    assert_eq!(_480p_video_transcode_state.codec, "avc1.640033".to_string());
                    assert_eq!(_480p_video_transcode_state.bitrate, 2000 * 1024);
                    assert!(!_480p_video_transcode_state.copy);
                    assert_eq!(
                        _480p_video_transcode_state.settings,
                        Some(stream_state::transcode::Settings::Video(
                            stream_state::transcode::VideoSettings {
                                framerate: 30,
                                height: 480,
                                width: 853,
                            }
                        ))
                    );

                    assert_eq!(aac_480p.transcode_ids[0], opus_480p.transcode_ids[0]);
                    assert_eq!(aac_480p.transcode_ids[1], aac_transcode_state.id);
                    assert_eq!(opus_480p.transcode_ids[1], opus_transcode_state.id);

                    let _360p_video_transcode_state = v
                        .transcodes
                        .iter()
                        .find(|s| s.id == aac_360p.transcode_ids[0])
                        .unwrap();
                    assert_eq!(_360p_video_transcode_state.codec, "avc1.640033".to_string());
                    assert_eq!(_360p_video_transcode_state.bitrate, 1000 * 1024);
                    assert!(!_360p_video_transcode_state.copy);
                    assert_eq!(
                        _360p_video_transcode_state.settings,
                        Some(stream_state::transcode::Settings::Video(
                            stream_state::transcode::VideoSettings {
                                framerate: 30,
                                height: 360,
                                width: 640,
                            }
                        ))
                    );

                    assert_eq!(aac_360p.transcode_ids[0], opus_360p.transcode_ids[0]);
                    assert_eq!(aac_360p.transcode_ids[1], aac_transcode_state.id);
                    assert_eq!(opus_360p.transcode_ids[1], opus_transcode_state.id);

                    stream_state = Some(v.clone());

                    send.send(Ok(UpdateLiveStreamResponse {})).unwrap();
                }
                _ => panic!("unexpected update"),
            }
        }
        _ => panic!("unexpected event"),
    }

    match state.api_recv().await {
        IncomingRequest::Update((update, response)) => {
            assert_eq!(update.stream_id, stream_id.to_string());
            assert_eq!(update.updates.len(), 1);

            let update = &update.updates[0];
            assert!(update.timestamp > 0);

            match &update.update {
                Some(update_live_stream_request::update::Update::Event(event)) => {
                    assert_eq!(event.title, "Requested Transcoder");
                    assert_eq!(
                        event.message,
                        "Requested a transcoder to be assigned to this stream"
                    );
                    assert_eq!(event.level, Level::Info as i32)
                }
                u => {
                    panic!("unexpected update: {:?}", u);
                }
            }

            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    let msg = state.transcoder_message().await;
    assert!(!msg.id.is_empty());
    assert!(msg.timestamp > 0);
    let data = match msg.data {
        Some(transcoder_message::Data::NewStream(data)) => data,
        _ => panic!("unexpected message"),
    };

    assert!(!data.request_id.is_empty());
    assert_eq!(data.stream_id, stream_id.to_string());
    assert_eq!(data.state, stream_state);

    // We should now be able to join the stream
    let stream_id = data.stream_id.parse().unwrap();
    let request_id = data.request_id.parse().unwrap();
    let mut watcher = Watcher::new(&state, stream_id, request_id).await;

    match watcher.recv().await {
        WatchStreamEvent::InitSegment(data) => assert!(!data.is_empty()),
        _ => panic!("unexpected event"),
    }

    match watcher.recv().await {
        WatchStreamEvent::MediaSegment(ms) => {
            assert!(!ms.data.is_empty());
            assert!(ms.keyframe);
        }
        _ => panic!("unexpected event"),
    }

    assert!(
        state
            .global
            .connection_manager
            .submit_request(stream_id, GrpcRequest::TranscoderStarted { id: request_id })
            .await
    );

    match state.api_recv().await {
        IncomingRequest::Update((update, response)) => {
            assert_eq!(update.stream_id, stream_id.to_string());
            assert_eq!(update.updates.len(), 1);

            let update = &update.updates[0];
            assert!(update.timestamp > 0);

            match &update.update {
                Some(update_live_stream_request::update::Update::ReadyState(state)) => {
                    assert_eq!(*state, StreamReadyState::Ready as i32); // Stream is ready
                }
                u => {
                    panic!("unexpected update: {:?}", u);
                }
            }

            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    // Finish the stream
    let mut got_shutting_down = false;
    while let Some(msg) = watcher.rx.recv().await {
        match msg {
            WatchStreamEvent::MediaSegment(ms) => {
                assert!(!ms.data.is_empty());
            }
            WatchStreamEvent::ShuttingDown(true) => {
                got_shutting_down = true;
                break;
            }
            _ => panic!("unexpected event"),
        }
    }

    assert!(got_shutting_down);

    tokio::time::sleep(Duration::from_millis(200)).await;

    assert!(ffmpeg.try_wait().is_ok());

    // Assert that the stream is removed
    assert!(
        !state
            .global
            .connection_manager
            .submit_request(stream_id, GrpcRequest::TranscoderStarted { id: request_id })
            .await
    );

    // Assert that the stream is removed
    match state.api_recv().await {
        IncomingRequest::Update((update, response)) => {
            assert_eq!(update.stream_id, stream_id.to_string());
            assert_eq!(update.updates.len(), 1);

            let update = &update.updates[0];
            assert!(update.timestamp > 0);

            match &update.update {
                Some(update_live_stream_request::update::Update::ReadyState(state)) => {
                    assert_eq!(*state, StreamReadyState::Stopped as i32); // graceful stop
                }
                u => {
                    panic!("unexpected update: {:?}", u);
                }
            }

            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    state.finish().await;
}

#[tokio::test]
async fn test_ingest_stream_reject() {
    let mut state = TestState::setup().await;
    let mut ffmpeg = stream_with_ffmpeg(state.rtmp_port, "avc_aac_large.mp4");

    let stream_id = Uuid::new_v4();
    state
        .api_assert_authenticate(Err(Status::permission_denied("invalid stream key")))
        .await;

    assert!(
        tokio::time::timeout(Duration::from_secs(1), state.transcoder_stream.next())
            .await
            .is_err()
    );

    tokio::time::sleep(Duration::from_millis(200)).await;

    assert!(ffmpeg.try_wait().is_ok());

    // Assert that the stream is removed
    assert!(
        !state
            .global
            .connection_manager
            .submit_request(
                stream_id,
                GrpcRequest::TranscoderStarted { id: Uuid::new_v4() }
            )
            .await
    );

    state.finish().await;
}

#[tokio::test]
async fn test_ingest_stream_transcoder_error() {
    let mut state = TestState::setup().await;
    let mut ffmpeg = stream_with_ffmpeg(state.rtmp_port, "avc_aac_large.mp4");

    let stream_id = state.api_assert_authenticate_ok(false, true).await;

    let stream_state;
    match state.api_recv().await {
        IncomingRequest::Update((request, send)) => {
            assert_eq!(request.stream_id, stream_id.to_string());
            match &request.updates[0].update {
                Some(
                    crate::pb::scuffle::backend::update_live_stream_request::update::Update::State(
                        v,
                    ),
                ) => {
                    stream_state = v.clone();

                    send.send(Ok(UpdateLiveStreamResponse {})).unwrap();
                }
                _ => panic!("unexpected update"),
            }
        }
        _ => panic!("unexpected event"),
    }

    match state.api_recv().await {
        IncomingRequest::Update((update, response)) => {
            assert_eq!(update.stream_id, stream_id.to_string());
            assert_eq!(update.updates.len(), 1);

            let update = &update.updates[0];
            assert!(update.timestamp > 0);

            match &update.update {
                Some(update_live_stream_request::update::Update::Event(event)) => {
                    assert_eq!(event.title, "Requested Transcoder");
                    assert_eq!(
                        event.message,
                        "Requested a transcoder to be assigned to this stream"
                    );
                    assert_eq!(event.level, Level::Info as i32)
                }
                u => {
                    panic!("unexpected update: {:?}", u);
                }
            }

            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    let msg = state.transcoder_message().await;
    assert!(!msg.id.is_empty());
    assert!(msg.timestamp > 0);
    let data = match msg.data {
        Some(transcoder_message::Data::NewStream(data)) => data,
        _ => panic!("unexpected message"),
    };

    assert!(!data.request_id.is_empty());
    assert_eq!(data.stream_id, stream_id.to_string());
    assert_eq!(data.state, Some(stream_state));

    // We should now be able to join the stream
    let stream_id = data.stream_id.parse().unwrap();
    let request_id = data.request_id.parse().unwrap();
    let mut watcher = Watcher::new(&state, stream_id, request_id).await;

    match watcher.recv().await {
        WatchStreamEvent::InitSegment(data) => assert!(!data.is_empty()),
        _ => panic!("unexpected event"),
    }

    match watcher.recv().await {
        WatchStreamEvent::MediaSegment(ms) => {
            assert!(!ms.data.is_empty());
            assert!(ms.keyframe);
        }
        _ => panic!("unexpected event"),
    }

    assert!(
        state
            .global
            .connection_manager
            .submit_request(
                stream_id,
                GrpcRequest::TranscoderError {
                    id: request_id,
                    message: "test".to_string(),
                    fatal: false,
                }
            )
            .await
    );

    match state.api_recv().await {
        IncomingRequest::Update((update, response)) => {
            assert_eq!(update.stream_id, stream_id.to_string());
            assert_eq!(update.updates.len(), 2);

            let u = &update.updates[0];
            assert!(u.timestamp > 0);

            match &u.update {
                Some(update_live_stream_request::update::Update::Event(ev)) => {
                    assert_eq!(ev.title, "Transcoder Error");
                    assert_eq!(ev.message, "test");
                    assert_eq!(ev.level, Level::Error as i32)
                }
                u => {
                    panic!("unexpected update: {:?}", u);
                }
            }

            let u = &update.updates[1];
            assert!(u.timestamp > 0);

            match &u.update {
                Some(update_live_stream_request::update::Update::ReadyState(s)) => {
                    assert_eq!(*s, StreamReadyState::Failed as i32);
                }
                u => {
                    panic!("unexpected update: {:?}", u);
                }
            }

            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    // Finish the stream
    let mut got_shutting_down = false;
    while let Some(msg) = watcher.rx.recv().await {
        match msg {
            WatchStreamEvent::MediaSegment(ms) => {
                assert!(!ms.data.is_empty());
            }
            WatchStreamEvent::ShuttingDown(true) => {
                got_shutting_down = true;
                break;
            }
            _ => {}
        }
    }

    assert!(got_shutting_down);

    tokio::time::sleep(Duration::from_millis(200)).await;

    assert!(ffmpeg.try_wait().is_ok());

    // Assert that the stream is removed
    assert!(
        !state
            .global
            .connection_manager
            .submit_request(stream_id, GrpcRequest::TranscoderStarted { id: request_id })
            .await
    );

    assert!(
        tokio::time::timeout(Duration::from_secs(1), state.api_rx.recv())
            .await
            .is_err()
    );

    state.finish().await;
}

#[tokio::test]
async fn test_ingest_stream_try_resume_success() {
    let mut state = TestState::setup().await;
    let mut ffmpeg = stream_with_ffmpeg(state.rtmp_port, "avc_aac_large.mp4");

    let stream_id = Uuid::new_v4();

    let audio_transcode_id = Uuid::new_v4();
    let source_transcode_id = Uuid::new_v4();

    let stream_state = StreamState {
        variants: vec![
            stream_state::Variant {
                group: "aac".to_string(),
                name: "source".to_string(),
                transcode_ids: vec![
                    source_transcode_id.to_string(),
                    audio_transcode_id.to_string(),
                ],
            },
            stream_state::Variant {
                group: "aac".to_string(),
                name: "audio-only".to_string(),
                transcode_ids: vec![audio_transcode_id.to_string()],
            },
        ],
        transcodes: vec![
            stream_state::Transcode {
                id: source_transcode_id.to_string(),
                codec: "avc1.640034".to_string(),
                bitrate: 1740285,
                copy: true,
                settings: Some(stream_state::transcode::Settings::Video(
                    stream_state::transcode::VideoSettings {
                        width: 3840,
                        height: 2160,
                        framerate: 60,
                    },
                )),
            },
            stream_state::Transcode {
                id: audio_transcode_id.to_string(),
                codec: "mp4a.40.2".to_string(),
                bitrate: 128 * 1024,
                copy: false,
                settings: Some(stream_state::transcode::Settings::Audio(
                    stream_state::transcode::AudioSettings {
                        channels: 2,
                        sample_rate: 48000,
                    },
                )),
            },
        ],
        groups: vec![stream_state::Group {
            name: "aac".to_string(),
            priority: 1,
        }],
    };

    state
        .api_assert_authenticate(Ok(AuthenticateLiveStreamResponse {
            stream_id: stream_id.to_string(),
            record: false,
            transcode: false,
            state: Some(stream_state.clone()),
        }))
        .await;

    match state.api_recv().await {
        IncomingRequest::Update((update, response)) => {
            assert_eq!(update.stream_id, stream_id.to_string());
            assert_eq!(update.updates.len(), 1);

            let update = &update.updates[0];
            assert!(update.timestamp > 0);

            match &update.update {
                Some(update_live_stream_request::update::Update::Event(event)) => {
                    assert_eq!(event.title, "Requested Transcoder");
                    assert_eq!(
                        event.message,
                        "Requested a transcoder to be assigned to this stream"
                    );
                    assert_eq!(event.level, Level::Info as i32)
                }
                u => {
                    panic!("unexpected update: {:?}", u);
                }
            }

            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    let msg = state.transcoder_message().await;
    assert!(!msg.id.is_empty());
    assert!(msg.timestamp > 0);
    let data = match msg.data {
        Some(transcoder_message::Data::NewStream(data)) => data,
        _ => panic!("unexpected message"),
    };

    assert!(!data.request_id.is_empty());
    assert_eq!(data.stream_id, stream_id.to_string());
    assert_eq!(data.state, Some(stream_state));

    // We should now be able to join the stream
    let stream_id = data.stream_id.parse().unwrap();
    let request_id = data.request_id.parse().unwrap();
    let mut watcher = Watcher::new(&state, stream_id, request_id).await;

    match watcher.recv().await {
        WatchStreamEvent::InitSegment(data) => assert!(!data.is_empty()),
        _ => panic!("unexpected event"),
    }

    match watcher.recv().await {
        WatchStreamEvent::MediaSegment(ms) => {
            assert!(!ms.data.is_empty());
            assert!(ms.keyframe);
        }
        _ => panic!("unexpected event"),
    }

    assert!(
        state
            .global
            .connection_manager
            .submit_request(stream_id, GrpcRequest::TranscoderStarted { id: request_id })
            .await
    );

    match state.api_recv().await {
        IncomingRequest::Update((update, response)) => {
            assert_eq!(update.stream_id, stream_id.to_string());
            assert_eq!(update.updates.len(), 1);

            let update = &update.updates[0];
            assert!(update.timestamp > 0);

            match &update.update {
                Some(update_live_stream_request::update::Update::ReadyState(state)) => {
                    assert_eq!(*state, StreamReadyState::Ready as i32); // Stream is ready
                }
                u => {
                    panic!("unexpected update: {:?}", u);
                }
            }

            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    // Finish the stream
    let mut got_shutting_down = false;
    while let Some(msg) = watcher.rx.recv().await {
        match msg {
            WatchStreamEvent::MediaSegment(ms) => {
                assert!(!ms.data.is_empty());
            }
            WatchStreamEvent::ShuttingDown(true) => {
                got_shutting_down = true;
                break;
            }
            _ => panic!("unexpected event"),
        }
    }

    assert!(got_shutting_down);

    tokio::time::sleep(Duration::from_millis(200)).await;

    assert!(ffmpeg.try_wait().is_ok());

    // Assert that the stream is removed
    assert!(
        !state
            .global
            .connection_manager
            .submit_request(stream_id, GrpcRequest::TranscoderStarted { id: request_id })
            .await
    );

    // Assert that the stream is removed
    match state.api_recv().await {
        IncomingRequest::Update((update, response)) => {
            assert_eq!(update.stream_id, stream_id.to_string());
            assert_eq!(update.updates.len(), 1);

            let update = &update.updates[0];
            assert!(update.timestamp > 0);

            match &update.update {
                Some(update_live_stream_request::update::Update::ReadyState(state)) => {
                    assert_eq!(*state, StreamReadyState::Stopped as i32); // graceful stop
                }
                u => {
                    panic!("unexpected update: {:?}", u);
                }
            }

            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    state.finish().await;
}

#[tokio::test]
async fn test_ingest_stream_try_resume_failed() {
    let mut state = TestState::setup().await;
    let mut ffmpeg = stream_with_ffmpeg(state.rtmp_port, "avc_aac_large.mp4");

    let mut stream_id = Uuid::new_v4();

    let audio_transcode_id = Uuid::new_v4();
    let source_transcode_id = Uuid::new_v4();

    state
        .api_assert_authenticate(Ok(AuthenticateLiveStreamResponse {
            stream_id: stream_id.to_string(),
            record: false,
            transcode: false,
            state: Some(StreamState {
                variants: vec![
                    stream_state::Variant {
                        group: "aac".to_string(),
                        name: "source".to_string(),
                        transcode_ids: vec![
                            source_transcode_id.to_string(),
                            audio_transcode_id.to_string(),
                        ],
                    },
                    stream_state::Variant {
                        group: "aac".to_string(),
                        name: "audio-only".to_string(),
                        transcode_ids: vec![audio_transcode_id.to_string()],
                    },
                ],
                transcodes: vec![
                    stream_state::Transcode {
                        id: source_transcode_id.to_string(),
                        codec: "avc1.640034".to_string(),
                        bitrate: 1740285,
                        copy: true,
                        settings: Some(stream_state::transcode::Settings::Video(
                            stream_state::transcode::VideoSettings {
                                width: 3840,
                                height: 2160,
                                framerate: 30, // Note we changed this to 30fps from 60 so that we could cause the stream to fail
                            },
                        )),
                    },
                    stream_state::Transcode {
                        id: audio_transcode_id.to_string(),
                        codec: "mp4a.40.2".to_string(),
                        bitrate: 128 * 1024,
                        copy: false,
                        settings: Some(stream_state::transcode::Settings::Audio(
                            stream_state::transcode::AudioSettings {
                                channels: 2,
                                sample_rate: 48000,
                            },
                        )),
                    },
                ],
                groups: vec![stream_state::Group {
                    name: "aac".to_string(),
                    priority: 1,
                }],
            }),
        }))
        .await;

    let stream_state;
    match state.api_recv().await {
        IncomingRequest::New((new, response)) => {
            assert_eq!(new.old_stream_id, stream_id.to_string());

            stream_state = Some(new.state.unwrap());

            stream_id = Uuid::new_v4();

            response
                .send(Ok(NewLiveStreamResponse {
                    stream_id: stream_id.to_string(),
                }))
                .unwrap();
        }
        _ => panic!("unexpected event"),
    }

    match state.api_recv().await {
        IncomingRequest::Update((update, response)) => {
            assert_eq!(update.stream_id, stream_id.to_string());
            assert_eq!(update.updates.len(), 1);

            let update = &update.updates[0];
            assert!(update.timestamp > 0);

            match &update.update {
                Some(update_live_stream_request::update::Update::Event(event)) => {
                    assert_eq!(event.title, "Requested Transcoder");
                    assert_eq!(
                        event.message,
                        "Requested a transcoder to be assigned to this stream"
                    );
                    assert_eq!(event.level, Level::Info as i32)
                }
                u => {
                    panic!("unexpected update: {:?}", u);
                }
            }

            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    let msg = state.transcoder_message().await;
    assert!(!msg.id.is_empty());
    assert!(msg.timestamp > 0);
    let data = match msg.data {
        Some(transcoder_message::Data::NewStream(data)) => data,
        _ => panic!("unexpected message"),
    };

    assert!(!data.request_id.is_empty());
    assert_eq!(data.stream_id, stream_id.to_string());
    assert_eq!(data.state, stream_state);

    // We should now be able to join the stream
    let stream_id = data.stream_id.parse().unwrap();
    let request_id = data.request_id.parse().unwrap();
    let mut watcher = Watcher::new(&state, stream_id, request_id).await;

    match watcher.recv().await {
        WatchStreamEvent::InitSegment(data) => assert!(!data.is_empty()),
        _ => panic!("unexpected event"),
    }

    match watcher.recv().await {
        WatchStreamEvent::MediaSegment(ms) => {
            assert!(!ms.data.is_empty());
            assert!(ms.keyframe);
        }
        _ => panic!("unexpected event"),
    }

    assert!(
        state
            .global
            .connection_manager
            .submit_request(stream_id, GrpcRequest::TranscoderStarted { id: request_id })
            .await
    );

    match state.api_recv().await {
        IncomingRequest::Update((update, response)) => {
            assert_eq!(update.stream_id, stream_id.to_string());
            assert_eq!(update.updates.len(), 1);

            let update = &update.updates[0];
            assert!(update.timestamp > 0);

            match &update.update {
                Some(update_live_stream_request::update::Update::ReadyState(state)) => {
                    assert_eq!(*state, StreamReadyState::Ready as i32); // Stream is ready
                }
                u => {
                    panic!("unexpected update: {:?}", u);
                }
            }

            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    // Finish the stream
    let mut got_shutting_down = false;
    while let Some(msg) = watcher.rx.recv().await {
        match msg {
            WatchStreamEvent::MediaSegment(ms) => {
                assert!(!ms.data.is_empty());
            }
            WatchStreamEvent::ShuttingDown(true) => {
                got_shutting_down = true;
                break;
            }
            _ => panic!("unexpected event"),
        }
    }

    assert!(got_shutting_down);

    tokio::time::sleep(Duration::from_millis(200)).await;

    assert!(ffmpeg.try_wait().is_ok());

    // Assert that the stream is removed
    assert!(
        !state
            .global
            .connection_manager
            .submit_request(stream_id, GrpcRequest::TranscoderStarted { id: request_id })
            .await
    );

    // Assert that the stream is removed
    match state.api_recv().await {
        IncomingRequest::Update((update, response)) => {
            assert_eq!(update.stream_id, stream_id.to_string());
            assert_eq!(update.updates.len(), 1);

            let update = &update.updates[0];
            assert!(update.timestamp > 0);

            match &update.update {
                Some(update_live_stream_request::update::Update::ReadyState(state)) => {
                    assert_eq!(*state, StreamReadyState::Stopped as i32); // graceful stop
                }
                u => {
                    panic!("unexpected update: {:?}", u);
                }
            }

            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    state.finish().await;
}

async fn test_ingest_stream_transcoder_full_tls(tls_dir: PathBuf) {
    let mut state = TestState::setup_with_tls(&tls_dir).await;
    let mut ffmpeg = stream_with_ffmpeg_tls(state.rtmp_port, "avc_aac_large.mp4", &tls_dir);

    let stream_id = Uuid::new_v4();
    match state.api_recv().await {
        IncomingRequest::Authenticate((request, send)) => {
            assert_eq!(request.stream_key, "stream-key");
            assert_eq!(request.app_name, "live");
            assert!(!request.connection_id.is_empty());
            assert!(!request.ingest_address.is_empty());
            send.send(Ok(AuthenticateLiveStreamResponse {
                stream_id: stream_id.to_string(),
                record: false,
                transcode: true,
                state: None,
            }))
            .unwrap();
        }
        _ => panic!("unexpected event"),
    }

    let stream_state;
    match state.api_recv().await {
        IncomingRequest::Update((request, send)) => {
            assert_eq!(request.stream_id, stream_id.to_string());
            match &request.updates[0].update {
                Some(
                    crate::pb::scuffle::backend::update_live_stream_request::update::Update::State(
                        v,
                    ),
                ) => {
                    stream_state = Some(v.clone());
                    send.send(Ok(UpdateLiveStreamResponse {})).unwrap();
                }
                _ => panic!("unexpected update"),
            }
        }
        _ => panic!("unexpected event"),
    }

    match state.api_recv().await {
        IncomingRequest::Update((update, response)) => {
            assert_eq!(update.stream_id, stream_id.to_string());
            assert_eq!(update.updates.len(), 1);

            let update = &update.updates[0];
            assert!(update.timestamp > 0);

            match &update.update {
                Some(update_live_stream_request::update::Update::Event(event)) => {
                    assert_eq!(event.title, "Requested Transcoder");
                    assert_eq!(
                        event.message,
                        "Requested a transcoder to be assigned to this stream"
                    );
                    assert_eq!(event.level, Level::Info as i32)
                }
                u => {
                    panic!("unexpected update: {:?}", u);
                }
            }

            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    let msg = state.transcoder_message().await;
    assert!(!msg.id.is_empty());
    assert!(msg.timestamp > 0);
    let data = match msg.data {
        Some(transcoder_message::Data::NewStream(data)) => data,
        _ => panic!("unexpected message"),
    };

    assert!(!data.request_id.is_empty());
    assert_eq!(data.stream_id, stream_id.to_string());
    assert_eq!(data.state, stream_state);

    // We should now be able to join the stream
    let stream_id = data.stream_id.parse().unwrap();
    let request_id = data.request_id.parse().unwrap();
    let mut watcher = Watcher::new(&state, stream_id, request_id).await;

    match watcher.recv().await {
        WatchStreamEvent::InitSegment(data) => assert!(!data.is_empty()),
        _ => panic!("unexpected event"),
    }

    match watcher.recv().await {
        WatchStreamEvent::MediaSegment(ms) => {
            assert!(!ms.data.is_empty());
            assert!(ms.keyframe);
        }
        _ => panic!("unexpected event"),
    }

    assert!(
        state
            .global
            .connection_manager
            .submit_request(stream_id, GrpcRequest::TranscoderStarted { id: request_id })
            .await
    );

    match state.api_recv().await {
        IncomingRequest::Update((update, response)) => {
            assert_eq!(update.stream_id, stream_id.to_string());
            assert_eq!(update.updates.len(), 1);

            let update = &update.updates[0];
            assert!(update.timestamp > 0);

            match &update.update {
                Some(update_live_stream_request::update::Update::ReadyState(state)) => {
                    assert_eq!(*state, StreamReadyState::Ready as i32); // Stream is ready
                }
                u => {
                    panic!("unexpected update: {:?}", u);
                }
            }

            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    // Finish the stream
    let mut got_shutting_down = false;
    while let Some(msg) = watcher.rx.recv().await {
        match msg {
            WatchStreamEvent::MediaSegment(ms) => {
                assert!(!ms.data.is_empty());
            }
            WatchStreamEvent::ShuttingDown(true) => {
                got_shutting_down = true;
                break;
            }
            _ => panic!("unexpected event"),
        }
    }

    assert!(got_shutting_down);

    tokio::time::sleep(Duration::from_millis(200)).await;

    assert!(ffmpeg.try_wait().is_ok());

    // Assert that the stream is removed
    assert!(
        !state
            .global
            .connection_manager
            .submit_request(stream_id, GrpcRequest::TranscoderStarted { id: request_id })
            .await
    );

    // Assert that the stream is removed
    match state.api_recv().await {
        IncomingRequest::Update((update, response)) => {
            assert_eq!(update.stream_id, stream_id.to_string());
            assert_eq!(update.updates.len(), 1);

            let update = &update.updates[0];
            assert!(update.timestamp > 0);

            match &update.update {
                Some(update_live_stream_request::update::Update::ReadyState(state)) => {
                    assert_eq!(*state, StreamReadyState::Stopped as i32); // graceful stop
                }
                u => {
                    panic!("unexpected update: {:?}", u);
                }
            }

            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    // state.finish().await;
}

#[tokio::test]
async fn test_ingest_stream_transcoder_full_tls_rsa() {
    test_ingest_stream_transcoder_full_tls(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/tests/certs/rsa"),
    )
    .await;
}

#[tokio::test]
async fn test_ingest_stream_transcoder_full_tls_ec() {
    test_ingest_stream_transcoder_full_tls(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/tests/certs/ec"),
    )
    .await;
}

#[tokio::test]
async fn test_ingest_stream_transcoder_probe() {
    let mut state = TestState::setup().await;
    let mut ffmpeg = stream_with_ffmpeg(state.rtmp_port, "avc_aac_keyframes.mp4");

    let stream_id = state.api_assert_authenticate_ok(false, false).await;

    match state.api_recv().await {
        IncomingRequest::Update((_, send)) => {
            send.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    match state.api_recv().await {
        IncomingRequest::Update((_, response)) => {
            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    let msg = state.transcoder_message().await;
    assert!(!msg.id.is_empty());
    assert!(msg.timestamp > 0);
    let data = match msg.data {
        Some(transcoder_message::Data::NewStream(data)) => data,
        _ => panic!("unexpected message"),
    };

    assert!(!data.request_id.is_empty());
    assert_eq!(data.stream_id, stream_id.to_string());

    // We should now be able to join the stream
    let stream_id = data.stream_id.parse().unwrap();
    let request_id = data.request_id.parse().unwrap();
    let mut watcher = Watcher::new(&state, stream_id, request_id).await;

    let mut ffprobe = spawn_ffprobe();
    let writer = ffprobe.stdin.as_mut().unwrap();

    match watcher.recv().await {
        WatchStreamEvent::InitSegment(data) => writer.write_all(&data).await.unwrap(),
        _ => panic!("unexpected event"),
    }

    match watcher.recv().await {
        WatchStreamEvent::MediaSegment(ms) => {
            writer.write_all(&ms.data).await.unwrap();
        }
        _ => panic!("unexpected event"),
    }

    assert!(
        state
            .global
            .connection_manager
            .submit_request(stream_id, GrpcRequest::TranscoderStarted { id: request_id })
            .await
    );

    match state.api_recv().await {
        IncomingRequest::Update((_, response)) => {
            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    // Finish the stream
    let mut got_shutting_down = false;
    while let Some(msg) = watcher.rx.recv().await {
        match msg {
            WatchStreamEvent::MediaSegment(ms) => {
                writer.write_all(&ms.data).await.unwrap();
            }
            WatchStreamEvent::ShuttingDown(true) => {
                got_shutting_down = true;
                break;
            }
            _ => panic!("unexpected event"),
        }
    }

    assert!(got_shutting_down);

    tokio::time::sleep(Duration::from_millis(200)).await;

    assert!(ffmpeg.try_wait().is_ok());

    let output = ffprobe.wait_with_output().await.unwrap();
    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    {
        let video_stream = &json["streams"][0];
        assert_eq!(video_stream["codec_type"], "video");
        assert_eq!(video_stream["codec_name"], "h264");
        assert_eq!(video_stream["width"], 480);
        assert_eq!(video_stream["height"], 852);
        assert_eq!(video_stream["r_frame_rate"], "30/1");
        assert_eq!(video_stream["avg_frame_rate"], "30/1");
        assert_eq!(video_stream["time_base"], "1/30000");
        assert_eq!(video_stream["codec_tag"], "0x31637661");
        assert_eq!(video_stream["codec_tag_string"], "avc1");
        assert_eq!(video_stream["profile"], "High");
        assert_eq!(video_stream["level"], 31);
        assert_eq!(video_stream["refs"], 1);
        assert_eq!(video_stream["is_avc"], "true");

        let audio_stream = &json["streams"][1];
        assert_eq!(audio_stream["codec_type"], "audio");
        assert_eq!(audio_stream["codec_name"], "aac");
        assert_eq!(audio_stream["sample_rate"], "44100");
        assert_eq!(audio_stream["channels"], 1);
        assert_eq!(audio_stream["channel_layout"], "mono");
        assert_eq!(audio_stream["r_frame_rate"], "0/0");
        assert_eq!(audio_stream["avg_frame_rate"], "0/0");
        assert_eq!(audio_stream["time_base"], "1/44100");
        assert_eq!(audio_stream["codec_tag"], "0x6134706d");
        assert_eq!(audio_stream["codec_tag_string"], "mp4a");
        assert_eq!(audio_stream["profile"], "LC");
    }

    // Assert that the stream is removed
    assert!(
        !state
            .global
            .connection_manager
            .submit_request(stream_id, GrpcRequest::TranscoderStarted { id: request_id })
            .await
    );

    // Assert that the stream is removed
    match state.api_recv().await {
        IncomingRequest::Update((update, response)) => {
            assert_eq!(update.stream_id, stream_id.to_string());
            assert_eq!(update.updates.len(), 1);

            let update = &update.updates[0];
            assert!(update.timestamp > 0);

            match &update.update {
                Some(update_live_stream_request::update::Update::ReadyState(state)) => {
                    assert_eq!(*state, StreamReadyState::Stopped as i32); // graceful stop
                }
                u => {
                    panic!("unexpected update: {:?}", u);
                }
            }

            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    state.finish().await;
}

#[tokio::test]
async fn test_ingest_stream_transcoder_probe_reconnect() {
    let mut state = TestState::setup().await;
    let mut ffmpeg = stream_with_ffmpeg(state.rtmp_port, "avc_aac_keyframes.mp4");

    let stream_id = state.api_assert_authenticate_ok(false, false).await;

    match state.api_recv().await {
        IncomingRequest::Update((_, send)) => {
            send.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    match state.api_recv().await {
        IncomingRequest::Update((_, response)) => {
            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    let msg = state.transcoder_message().await;
    assert!(!msg.id.is_empty());
    assert!(msg.timestamp > 0);
    let data = match msg.data {
        Some(transcoder_message::Data::NewStream(data)) => data,
        _ => panic!("unexpected message"),
    };

    assert!(!data.request_id.is_empty());
    assert_eq!(data.stream_id, stream_id.to_string());

    // We should now be able to join the stream
    let stream_id = data.stream_id.parse().unwrap();
    let request_id = data.request_id.parse().unwrap();
    let mut watcher = Watcher::new(&state, stream_id, request_id).await;

    let mut ffprobe = spawn_ffprobe();
    let writer = ffprobe.stdin.as_mut().unwrap();

    match watcher.recv().await {
        WatchStreamEvent::InitSegment(data) => writer.write_all(&data).await.unwrap(),
        _ => panic!("unexpected event"),
    }

    assert!(
        state
            .global
            .connection_manager
            .submit_request(stream_id, GrpcRequest::TranscoderStarted { id: request_id })
            .await
    );

    match state.api_recv().await {
        IncomingRequest::Update((_, response)) => {
            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    // Finish the stream
    let mut i = 0;
    while let Some(msg) = watcher.rx.recv().await {
        match msg {
            WatchStreamEvent::MediaSegment(ms) => {
                writer.write_all(&ms.data).await.unwrap();
            }
            _ => panic!("unexpected event"),
        }
        i += 1;

        if i > 10 {
            break;
        }
    }

    let output = ffprobe.wait_with_output().await.unwrap();
    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    {
        let video_stream = &json["streams"][0];
        assert_eq!(video_stream["codec_type"], "video");
        assert_eq!(video_stream["codec_name"], "h264");
        assert_eq!(video_stream["width"], 480);
        assert_eq!(video_stream["height"], 852);
        assert_eq!(video_stream["r_frame_rate"], "30/1");
        assert_eq!(video_stream["avg_frame_rate"], "30/1");
        assert_eq!(video_stream["time_base"], "1/30000");
        assert_eq!(video_stream["codec_tag"], "0x31637661");
        assert_eq!(video_stream["codec_tag_string"], "avc1");
        assert_eq!(video_stream["profile"], "High");
        assert_eq!(video_stream["level"], 31);
        assert_eq!(video_stream["refs"], 1);
        assert_eq!(video_stream["is_avc"], "true");

        let audio_stream = &json["streams"][1];
        assert_eq!(audio_stream["codec_type"], "audio");
        assert_eq!(audio_stream["codec_name"], "aac");
        assert_eq!(audio_stream["sample_rate"], "44100");
        assert_eq!(audio_stream["channels"], 1);
        assert_eq!(audio_stream["channel_layout"], "mono");
        assert_eq!(audio_stream["r_frame_rate"], "0/0");
        assert_eq!(audio_stream["avg_frame_rate"], "0/0");
        assert_eq!(audio_stream["time_base"], "1/44100");
        assert_eq!(audio_stream["codec_tag"], "0x6134706d");
        assert_eq!(audio_stream["codec_tag_string"], "mp4a");
        assert_eq!(audio_stream["profile"], "LC");
    }

    assert!(
        state
            .global
            .connection_manager
            .submit_request(
                stream_id,
                GrpcRequest::TranscoderShuttingDown { id: request_id }
            )
            .await
    );

    match state.api_recv().await {
        IncomingRequest::Update((_, response)) => {
            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    let msg = state.transcoder_message().await;
    assert!(!msg.id.is_empty());
    assert!(msg.timestamp > 0);
    let data = match msg.data {
        Some(transcoder_message::Data::NewStream(data)) => data,
        _ => panic!("unexpected message"),
    };

    assert!(!data.request_id.is_empty());
    assert_eq!(data.stream_id, stream_id.to_string());

    // We should now be able to join the stream
    let stream_id = data.stream_id.parse().unwrap();
    let request_id = data.request_id.parse().unwrap();
    let mut new_watcher = Watcher::new(&state, stream_id, request_id).await;

    let mut got_shutting_down = false;
    while let Some(msg) = watcher.rx.recv().await {
        match msg {
            WatchStreamEvent::MediaSegment(_) => {}
            WatchStreamEvent::ShuttingDown(false) => {
                got_shutting_down = true;
                break;
            }
            _ => panic!("unexpected event: {:?}", msg),
        }
    }

    assert!(got_shutting_down);

    let mut ffprobe = spawn_ffprobe();
    let writer = ffprobe.stdin.as_mut().unwrap();

    match new_watcher.recv().await {
        WatchStreamEvent::InitSegment(data) => writer.write_all(&data).await.unwrap(),
        _ => panic!("unexpected event"),
    }

    assert!(
        state
            .global
            .connection_manager
            .submit_request(stream_id, GrpcRequest::TranscoderStarted { id: request_id })
            .await
    );

    match state.api_recv().await {
        IncomingRequest::Update((_, response)) => {
            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    // Finish the stream
    let mut got_shutting_down = false;
    while let Some(msg) = new_watcher.rx.recv().await {
        match msg {
            WatchStreamEvent::MediaSegment(ms) => {
                writer.write_all(&ms.data).await.unwrap();
            }
            WatchStreamEvent::ShuttingDown(true) => {
                got_shutting_down = true;
                break;
            }
            _ => panic!("unexpected event"),
        }
    }

    assert!(got_shutting_down);

    let output = ffprobe.wait_with_output().await.unwrap();
    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    {
        let video_stream = &json["streams"][0];
        assert_eq!(video_stream["codec_type"], "video");
        assert_eq!(video_stream["codec_name"], "h264");
        assert_eq!(video_stream["width"], 480);
        assert_eq!(video_stream["height"], 852);
        assert_eq!(video_stream["r_frame_rate"], "30/1");
        assert_eq!(video_stream["avg_frame_rate"], "30/1");
        assert_eq!(video_stream["time_base"], "1/30000");
        assert_eq!(video_stream["codec_tag"], "0x31637661");
        assert_eq!(video_stream["codec_tag_string"], "avc1");
        assert_eq!(video_stream["profile"], "High");
        assert_eq!(video_stream["level"], 31);
        assert_eq!(video_stream["refs"], 1);
        assert_eq!(video_stream["is_avc"], "true");

        let audio_stream = &json["streams"][1];
        assert_eq!(audio_stream["codec_type"], "audio");
        assert_eq!(audio_stream["codec_name"], "aac");
        assert_eq!(audio_stream["sample_rate"], "44100");
        assert_eq!(audio_stream["channels"], 1);
        assert_eq!(audio_stream["channel_layout"], "mono");
        assert_eq!(audio_stream["r_frame_rate"], "0/0");
        assert_eq!(audio_stream["avg_frame_rate"], "0/0");
        assert_eq!(audio_stream["time_base"], "1/44100");
        assert_eq!(audio_stream["codec_tag"], "0x6134706d");
        assert_eq!(audio_stream["codec_tag_string"], "mp4a");
        assert_eq!(audio_stream["profile"], "LC");
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    assert!(ffmpeg.try_wait().is_ok());

    // Assert that the stream is removed
    assert!(
        !state
            .global
            .connection_manager
            .submit_request(stream_id, GrpcRequest::TranscoderStarted { id: request_id })
            .await
    );

    // Assert that the stream is removed
    match state.api_recv().await {
        IncomingRequest::Update((update, response)) => {
            assert_eq!(update.stream_id, stream_id.to_string());
            assert_eq!(update.updates.len(), 1);

            let update = &update.updates[0];
            assert!(update.timestamp > 0);

            match &update.update {
                Some(update_live_stream_request::update::Update::ReadyState(state)) => {
                    assert_eq!(*state, StreamReadyState::Stopped as i32); // graceful stop
                }
                u => {
                    panic!("unexpected update: {:?}", u);
                }
            }

            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    state.finish().await;
}

#[tokio::test]
async fn test_ingest_stream_transcoder_probe_reconnect_unexpected() {
    let mut state = TestState::setup().await;
    let mut ffmpeg = stream_with_ffmpeg(state.rtmp_port, "avc_aac_keyframes.mp4");

    let stream_id = state.api_assert_authenticate_ok(false, false).await;

    match state.api_recv().await {
        IncomingRequest::Update((_, send)) => {
            send.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    match state.api_recv().await {
        IncomingRequest::Update((_, response)) => {
            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    let msg = state.transcoder_message().await;
    assert!(!msg.id.is_empty());
    assert!(msg.timestamp > 0);
    let data = match msg.data {
        Some(transcoder_message::Data::NewStream(data)) => data,
        _ => panic!("unexpected message"),
    };

    assert!(!data.request_id.is_empty());
    assert_eq!(data.stream_id, stream_id.to_string());

    // We should now be able to join the stream
    let stream_id = data.stream_id.parse().unwrap();
    let request_id = data.request_id.parse().unwrap();
    let mut watcher = Watcher::new(&state, stream_id, request_id).await;

    let mut ffprobe = spawn_ffprobe();
    let writer = ffprobe.stdin.as_mut().unwrap();

    match watcher.recv().await {
        WatchStreamEvent::InitSegment(data) => writer.write_all(&data).await.unwrap(),
        _ => panic!("unexpected event"),
    }

    assert!(
        state
            .global
            .connection_manager
            .submit_request(stream_id, GrpcRequest::TranscoderStarted { id: request_id })
            .await
    );

    match state.api_recv().await {
        IncomingRequest::Update((_, response)) => {
            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    // Finish the stream
    let mut i = 0;
    while let Some(msg) = watcher.rx.recv().await {
        match msg {
            WatchStreamEvent::MediaSegment(ms) => {
                writer.write_all(&ms.data).await.unwrap();
            }
            _ => panic!("unexpected event"),
        }
        i += 1;

        if i > 10 {
            break;
        }
    }

    let output = ffprobe.wait_with_output().await.unwrap();
    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    {
        let video_stream = &json["streams"][0];
        assert_eq!(video_stream["codec_type"], "video");
        assert_eq!(video_stream["codec_name"], "h264");
        assert_eq!(video_stream["width"], 480);
        assert_eq!(video_stream["height"], 852);
        assert_eq!(video_stream["r_frame_rate"], "30/1");
        assert_eq!(video_stream["avg_frame_rate"], "30/1");
        assert_eq!(video_stream["time_base"], "1/30000");
        assert_eq!(video_stream["codec_tag"], "0x31637661");
        assert_eq!(video_stream["codec_tag_string"], "avc1");
        assert_eq!(video_stream["profile"], "High");
        assert_eq!(video_stream["level"], 31);
        assert_eq!(video_stream["refs"], 1);
        assert_eq!(video_stream["is_avc"], "true");

        let audio_stream = &json["streams"][1];
        assert_eq!(audio_stream["codec_type"], "audio");
        assert_eq!(audio_stream["codec_name"], "aac");
        assert_eq!(audio_stream["sample_rate"], "44100");
        assert_eq!(audio_stream["channels"], 1);
        assert_eq!(audio_stream["channel_layout"], "mono");
        assert_eq!(audio_stream["r_frame_rate"], "0/0");
        assert_eq!(audio_stream["avg_frame_rate"], "0/0");
        assert_eq!(audio_stream["time_base"], "1/44100");
        assert_eq!(audio_stream["codec_tag"], "0x6134706d");
        assert_eq!(audio_stream["codec_tag_string"], "mp4a");
        assert_eq!(audio_stream["profile"], "LC");
    }

    // Now drop the stream
    drop(watcher);

    match state.api_recv().await {
        IncomingRequest::Update((_, response)) => {
            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    match state.api_recv().await {
        IncomingRequest::Update((_, response)) => {
            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    let msg = state.transcoder_message().await;
    assert!(!msg.id.is_empty());
    assert!(msg.timestamp > 0);
    let data = match msg.data {
        Some(transcoder_message::Data::NewStream(data)) => data,
        _ => panic!("unexpected message"),
    };

    assert!(!data.request_id.is_empty());
    assert_eq!(data.stream_id, stream_id.to_string());

    // We should now be able to join the stream
    let stream_id = data.stream_id.parse().unwrap();
    let request_id = data.request_id.parse().unwrap();
    let mut watcher = Watcher::new(&state, stream_id, request_id).await;

    let mut ffprobe = spawn_ffprobe();
    let writer = ffprobe.stdin.as_mut().unwrap();

    match watcher.recv().await {
        WatchStreamEvent::InitSegment(data) => writer.write_all(&data).await.unwrap(),
        _ => panic!("unexpected event"),
    }

    assert!(
        state
            .global
            .connection_manager
            .submit_request(stream_id, GrpcRequest::TranscoderStarted { id: request_id })
            .await
    );

    match state.api_recv().await {
        IncomingRequest::Update((_, response)) => {
            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    // Finish the stream
    let mut got_shutting_down = false;
    while let Some(msg) = watcher.rx.recv().await {
        match msg {
            WatchStreamEvent::MediaSegment(ms) => {
                writer.write_all(&ms.data).await.unwrap();
            }
            WatchStreamEvent::ShuttingDown(true) => {
                got_shutting_down = true;
                break;
            }
            _ => panic!("unexpected event"),
        }
    }

    assert!(got_shutting_down);

    let output = ffprobe.wait_with_output().await.unwrap();
    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    {
        let video_stream = &json["streams"][0];
        assert_eq!(video_stream["codec_type"], "video");
        assert_eq!(video_stream["codec_name"], "h264");
        assert_eq!(video_stream["width"], 480);
        assert_eq!(video_stream["height"], 852);
        assert_eq!(video_stream["r_frame_rate"], "30/1");
        assert_eq!(video_stream["avg_frame_rate"], "30/1");
        assert_eq!(video_stream["time_base"], "1/30000");
        assert_eq!(video_stream["codec_tag"], "0x31637661");
        assert_eq!(video_stream["codec_tag_string"], "avc1");
        assert_eq!(video_stream["profile"], "High");
        assert_eq!(video_stream["level"], 31);
        assert_eq!(video_stream["refs"], 1);
        assert_eq!(video_stream["is_avc"], "true");

        let audio_stream = &json["streams"][1];
        assert_eq!(audio_stream["codec_type"], "audio");
        assert_eq!(audio_stream["codec_name"], "aac");
        assert_eq!(audio_stream["sample_rate"], "44100");
        assert_eq!(audio_stream["channels"], 1);
        assert_eq!(audio_stream["channel_layout"], "mono");
        assert_eq!(audio_stream["r_frame_rate"], "0/0");
        assert_eq!(audio_stream["avg_frame_rate"], "0/0");
        assert_eq!(audio_stream["time_base"], "1/44100");
        assert_eq!(audio_stream["codec_tag"], "0x6134706d");
        assert_eq!(audio_stream["codec_tag_string"], "mp4a");
        assert_eq!(audio_stream["profile"], "LC");
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    assert!(ffmpeg.try_wait().is_ok());

    // Assert that the stream is removed
    assert!(
        !state
            .global
            .connection_manager
            .submit_request(stream_id, GrpcRequest::TranscoderStarted { id: request_id })
            .await
    );

    // Assert that the stream is removed
    match state.api_recv().await {
        IncomingRequest::Update((update, response)) => {
            assert_eq!(update.stream_id, stream_id.to_string());
            assert_eq!(update.updates.len(), 1);

            let update = &update.updates[0];
            assert!(update.timestamp > 0);

            match &update.update {
                Some(update_live_stream_request::update::Update::ReadyState(state)) => {
                    assert_eq!(*state, StreamReadyState::Stopped as i32); // graceful stop
                }
                u => {
                    panic!("unexpected update: {:?}", u);
                }
            }

            response.send(Ok(UpdateLiveStreamResponse {})).unwrap();
        }
        _ => panic!("unexpected event"),
    }

    state.finish().await;
}
