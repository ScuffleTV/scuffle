// TODO: This is the test stub for the transcoder service. It is not yet implemented.
#![allow(unused_imports)]
#![allow(dead_code)]

use std::{
    collections::HashMap, io::Cursor, net::SocketAddr, path::PathBuf, pin::Pin, sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use bytes::{Buf, Bytes};
use chrono::Utc;
use fred::prelude::{HashesInterface, KeysInterface};
use futures_util::Stream;
use lapin::BasicProperties;
use mp4::DynBox;
use prost::Message;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response};
use transmuxer::{MediaType, TransmuxResult, Transmuxer};
use uuid::Uuid;

use crate::{
    config::{AppConfig, LoggingConfig, RmqConfig},
    global::{self, GlobalState},
    pb::scuffle::{
        events::{self, transcoder_message},
        types::{stream_state, StreamState},
        video::{
            ingest_server::{Ingest, IngestServer},
            transcoder_event_request, watch_stream_response, ShutdownStreamRequest,
            ShutdownStreamResponse, TranscoderEventRequest, TranscoderEventResponse,
            WatchStreamRequest, WatchStreamResponse,
        },
    },
    transcoder::{
        self,
        job::variant::state::{PlaylistState, SegmentState},
    },
};

struct ImplIngestServer {
    tx: mpsc::Sender<IngestRequest>,
}

#[derive(Debug)]
enum IngestRequest {
    WatchStream {
        request: WatchStreamRequest,
        tx: mpsc::Sender<Result<WatchStreamResponse>>,
    },
    TranscoderEvent {
        request: TranscoderEventRequest,
        tx: oneshot::Sender<TranscoderEventResponse>,
    },
    Shutdown {
        request: ShutdownStreamRequest,
        tx: oneshot::Sender<ShutdownStreamResponse>,
    },
}

type Result<T> = std::result::Result<T, tonic::Status>;

#[async_trait]
impl Ingest for ImplIngestServer {
    type WatchStreamStream =
        Pin<Box<dyn Stream<Item = Result<WatchStreamResponse>> + 'static + Send>>;

    async fn watch_stream(
        &self,
        request: tonic::Request<WatchStreamRequest>,
    ) -> Result<Response<Self::WatchStreamStream>> {
        let (tx, rx) = mpsc::channel(256);
        let request = IngestRequest::WatchStream {
            request: request.into_inner(),
            tx,
        };
        self.tx.send(request).await.unwrap();
        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    async fn transcoder_event(
        &self,
        request: Request<TranscoderEventRequest>,
    ) -> Result<Response<TranscoderEventResponse>> {
        let (tx, rx) = oneshot::channel();
        let request = IngestRequest::TranscoderEvent {
            request: request.into_inner(),
            tx,
        };

        self.tx.send(request).await.unwrap();
        Ok(Response::new(rx.await.unwrap()))
    }

    async fn shutdown_stream(
        &self,
        request: Request<ShutdownStreamRequest>,
    ) -> Result<Response<ShutdownStreamResponse>> {
        let (tx, rx) = oneshot::channel();
        let request = IngestRequest::Shutdown {
            request: request.into_inner(),
            tx,
        };

        self.tx.send(request).await.unwrap();
        Ok(Response::new(rx.await.unwrap()))
    }
}

fn setup_ingest_server(
    global: Arc<GlobalState>,
    bind: impl Into<SocketAddr>,
) -> mpsc::Receiver<IngestRequest> {
    let (tx, rx) = mpsc::channel(256);
    let server = ImplIngestServer { tx };
    let bind = bind.into();

    tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(IngestServer::new(server))
            .serve_with_shutdown(bind, async move {
                global.ctx.done().await;
            })
            .await
            .unwrap();
    });

    rx
}

#[tokio::test]
async fn test_transcode() {
    let port = portpicker::pick_unused_port().unwrap();

    let (global, handler) = crate::tests::global::mock_global_state(AppConfig {
        rmq: RmqConfig {
            transcoder_queue: Uuid::new_v4().to_string(),
            uri: "".to_string(),
        },
        logging: LoggingConfig {
            level: "info,transcoder=debug".to_string(),
            json: false,
        },
        ..Default::default()
    })
    .await;

    global::init_rmq(&global, true).await;

    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let mut rx = setup_ingest_server(global.clone(), addr);

    let transcoder_run_handle = tokio::spawn(transcoder::run(global.clone()));

    let channel = global.rmq.aquire().await.unwrap();

    let req_id = Uuid::new_v4();

    let source_video_id = Uuid::new_v4();
    let aac_audio_id = Uuid::new_v4();
    let opus_audio_id = Uuid::new_v4();
    let video_id_360p = Uuid::new_v4();

    channel
        .basic_publish(
            "",
            &global.config.rmq.transcoder_queue,
            lapin::options::BasicPublishOptions::default(),
            events::TranscoderMessage {
                id: req_id.to_string(),
                timestamp: Utc::now().timestamp() as u64,
                data: Some(transcoder_message::Data::NewStream(
                    events::TranscoderMessageNewStream {
                        request_id: req_id.to_string(),
                        stream_id: req_id.to_string(),
                        ingest_address: addr.to_string(),
                        state: Some(StreamState {
                            transcodes: vec![
                                stream_state::Transcode {
                                    bitrate: 1000,
                                    codec: "avc1.64002a".to_string(),
                                    id: source_video_id.to_string(),
                                    copy: true,
                                    settings: Some(stream_state::transcode::Settings::Video(
                                        stream_state::transcode::VideoSettings {
                                            framerate: 30,
                                            height: 1080,
                                            width: 1920,
                                        },
                                    )),
                                },
                                stream_state::Transcode {
                                    bitrate: 1024 * 1024,
                                    codec: "avc1.64002a".to_string(),
                                    id: video_id_360p.to_string(),
                                    copy: false,
                                    settings: Some(stream_state::transcode::Settings::Video(
                                        stream_state::transcode::VideoSettings {
                                            framerate: 30,
                                            height: 360,
                                            width: 640,
                                        },
                                    )),
                                },
                                stream_state::Transcode {
                                    bitrate: 96 * 1024,
                                    codec: "opus".to_string(),
                                    id: opus_audio_id.to_string(),
                                    copy: false,
                                    settings: Some(stream_state::transcode::Settings::Audio(
                                        stream_state::transcode::AudioSettings {
                                            channels: 2,
                                            sample_rate: 48000,
                                        },
                                    )),
                                },
                                stream_state::Transcode {
                                    bitrate: 96 * 1024,
                                    codec: "mp4a.40.2".to_string(),
                                    id: aac_audio_id.to_string(),
                                    copy: false,
                                    settings: Some(stream_state::transcode::Settings::Audio(
                                        stream_state::transcode::AudioSettings {
                                            channels: 2,
                                            sample_rate: 48000,
                                        },
                                    )),
                                },
                            ],
                            variants: vec![
                                stream_state::Variant {
                                    name: "source".to_string(),
                                    group: "aac".to_string(),
                                    transcode_ids: vec![
                                        source_video_id.to_string(),
                                        aac_audio_id.to_string(),
                                    ],
                                },
                                stream_state::Variant {
                                    name: "source".to_string(),
                                    group: "opus".to_string(),
                                    transcode_ids: vec![
                                        source_video_id.to_string(),
                                        opus_audio_id.to_string(),
                                    ],
                                },
                                stream_state::Variant {
                                    name: "360p".to_string(),
                                    group: "aac".to_string(),
                                    transcode_ids: vec![
                                        video_id_360p.to_string(),
                                        aac_audio_id.to_string(),
                                    ],
                                },
                                stream_state::Variant {
                                    name: "360p".to_string(),
                                    group: "opus".to_string(),
                                    transcode_ids: vec![
                                        video_id_360p.to_string(),
                                        opus_audio_id.to_string(),
                                    ],
                                },
                                stream_state::Variant {
                                    name: "audio-only".to_string(),
                                    group: "aac".to_string(),
                                    transcode_ids: vec![aac_audio_id.to_string()],
                                },
                                stream_state::Variant {
                                    name: "audio-only".to_string(),
                                    group: "opus".to_string(),
                                    transcode_ids: vec![opus_audio_id.to_string()],
                                },
                            ],
                            groups: vec![
                                stream_state::Group {
                                    name: "opus".to_string(),
                                    priority: 1,
                                },
                                stream_state::Group {
                                    name: "aac".to_string(),
                                    priority: 2,
                                },
                            ],
                        }),
                    },
                )),
            }
            .encode_to_vec()
            .as_slice(),
            BasicProperties::default()
                .with_message_id(req_id.to_string().into())
                .with_content_type("application/octet-stream".into())
                .with_expiration("60000".into()),
        )
        .await
        .unwrap();

    let watch_stream_req = match rx.recv().await.unwrap() {
        IngestRequest::WatchStream { request, tx } => {
            assert_eq!(request.stream_id, req_id.to_string());
            assert_eq!(request.request_id, req_id.to_string());

            tx
        }
        _ => panic!("unexpected request"),
    };

    // This is now a stream we can write frames to.
    // We now need to demux the video into fragmnts to send to the transcoder.
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets");
    let data = std::fs::read(dir.join("avc_aac.flv").to_str().unwrap()).unwrap();

    let mut cursor = Cursor::new(Bytes::from(data));
    let mut transmuxer = Transmuxer::new();

    let flv = flv::Flv::demux(&mut cursor).unwrap();

    for tag in flv.tags {
        transmuxer.add_tag(tag);
        tracing::debug!("added tag");
        // We dont want to send too many frames at once, so we sleep a bit.
        tokio::time::sleep(Duration::from_millis(10)).await;
        if let Some(data) = transmuxer.mux().unwrap() {
            match data {
                TransmuxResult::InitSegment { data, .. } => {
                    watch_stream_req
                        .send(Ok(WatchStreamResponse {
                            data: Some(watch_stream_response::Data::InitSegment(data)),
                        }))
                        .await
                        .unwrap();
                }
                TransmuxResult::MediaSegment(ms) => {
                    watch_stream_req
                        .send(Ok(WatchStreamResponse {
                            data: Some(watch_stream_response::Data::MediaSegment(
                                watch_stream_response::MediaSegment {
                                    timestamp: ms.timestamp,
                                    data: ms.data,
                                    keyframe: ms.keyframe,
                                    data_type: match ms.ty {
                                        MediaType::Audio => {
                                            watch_stream_response::media_segment::DataType::Audio
                                                as i32
                                        }
                                        MediaType::Video => {
                                            watch_stream_response::media_segment::DataType::Video
                                                as i32
                                        }
                                    },
                                },
                            )),
                        }))
                        .await
                        .unwrap();
                }
            }
        }
    }

    match rx.recv().await.unwrap() {
        IngestRequest::TranscoderEvent { request, tx } => {
            assert_eq!(request.stream_id, req_id.to_string());
            assert_eq!(request.request_id, req_id.to_string());

            assert_eq!(
                request.event,
                Some(transcoder_event_request::Event::Started(true))
            );

            tx.send(TranscoderEventResponse {}).unwrap();
        }
        _ => panic!("unexpected request"),
    };

    tracing::debug!("finished sending frames");

    watch_stream_req
        .send(Ok(WatchStreamResponse {
            data: Some(watch_stream_response::Data::ShuttingDown(true)),
        }))
        .await
        .unwrap();

    let redis = global.redis.clone();
    drop(watch_stream_req);
    drop(global);
    handler.cancel().await;
    transcoder_run_handle.await.unwrap().unwrap();

    // Validate data
    let resp: String = redis
        .get(format!("transcoder:{}:playlist", req_id))
        .await
        .unwrap();

    // Assert that the master playlist is correct.
    assert_eq!(
        resp,
        format!(
            r#"#EXTM3U
#EXT-X-MEDIA:TYPE=VIDEO,GROUP-ID="{source_video_id}",NAME="{source_video_id}",AUTOSELECT=YES,DEFAULT=YES,URI="{source_video_id}/index.m3u8"
#EXT-X-MEDIA:TYPE=VIDEO,GROUP-ID="{video_id_360p}",NAME="{video_id_360p}",AUTOSELECT=YES,DEFAULT=YES,URI="{video_id_360p}/index.m3u8"
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID="{opus_audio_id}",NAME="{opus_audio_id}",AUTOSELECT=YES,DEFAULT=YES,URI="{opus_audio_id}/index.m3u8"
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID="{aac_audio_id}",NAME="{aac_audio_id}",AUTOSELECT=YES,DEFAULT=YES,URI="{aac_audio_id}/index.m3u8"
#EXT-X-STREAM-INF:GROUP="aac",NAME="source",BANDWIDTH=99304,CODECS="avc1.64002a,mp4a.40.2",RESOLUTION=1920x1080,FRAME-RATE=30,VIDEO="{source_video_id}",AUDIO="{aac_audio_id}"
{source_video_id}/index.m3u8
#EXT-X-STREAM-INF:GROUP="opus",NAME="source",BANDWIDTH=99304,CODECS="avc1.64002a,opus",RESOLUTION=1920x1080,FRAME-RATE=30,VIDEO="{source_video_id}",AUDIO="{opus_audio_id}"
{source_video_id}/index.m3u8
#EXT-X-STREAM-INF:GROUP="aac",NAME="360p",BANDWIDTH=1146880,CODECS="avc1.64002a,mp4a.40.2",RESOLUTION=640x360,FRAME-RATE=30,VIDEO="{video_id_360p}",AUDIO="{aac_audio_id}"
{video_id_360p}/index.m3u8
#EXT-X-STREAM-INF:GROUP="opus",NAME="360p",BANDWIDTH=1146880,CODECS="avc1.64002a,opus",RESOLUTION=640x360,FRAME-RATE=30,VIDEO="{video_id_360p}",AUDIO="{opus_audio_id}"
{video_id_360p}/index.m3u8
#EXT-X-STREAM-INF:GROUP="aac",NAME="audio-only",BANDWIDTH=98304,CODECS="mp4a.40.2",AUDIO="{aac_audio_id}"
{aac_audio_id}/index.m3u8
#EXT-X-STREAM-INF:GROUP="opus",NAME="audio-only",BANDWIDTH=98304,CODECS="opus",AUDIO="{opus_audio_id}"
{opus_audio_id}/index.m3u8
#EXT-X-SCUF-GROUP:GROUP="opus",PRIORITY=1
#EXT-X-SCUF-GROUP:GROUP="aac",PRIORITY=2
"#
        )
    );

    let source_state: HashMap<String, String> = redis
        .hgetall(format!("transcoder:{}:{}:state", req_id, source_video_id))
        .await
        .unwrap();
    let source_state = PlaylistState::from(source_state);

    assert_eq!(source_state.current_fragment_idx(), 0);
    assert_eq!(source_state.current_segment_idx(), 1);
    assert_eq!(source_state.discontinuity_sequence(), 0);
    assert_eq!(source_state.track_count(), 1);
    assert_eq!(source_state.track_duration(0), Some(59000));
    assert_eq!(source_state.track_timescale(0), Some(60000));
    assert_eq!(source_state.longest_segment(), 59000.0 / 60000.0);

    let video_360p_state: HashMap<String, String> = redis
        .hgetall(format!("transcoder:{}:{}:state", req_id, video_id_360p))
        .await
        .unwrap();
    let video_360p_state = PlaylistState::from(video_360p_state);

    assert_eq!(video_360p_state.current_fragment_idx(), 0);
    assert_eq!(video_360p_state.current_segment_idx(), 1);
    assert_eq!(video_360p_state.discontinuity_sequence(), 0);
    assert_eq!(video_360p_state.track_count(), 1);
    assert_eq!(video_360p_state.track_duration(0), Some(15872));
    assert_eq!(video_360p_state.track_timescale(0), Some(15360));
    assert_eq!(video_360p_state.longest_segment(), 15872.0 / 15360.0);

    let opus_audio_state: HashMap<String, String> = redis
        .hgetall(format!("transcoder:{}:{}:state", req_id, opus_audio_id))
        .await
        .unwrap();
    let opus_audio_state = PlaylistState::from(opus_audio_state);

    assert_eq!(opus_audio_state.current_fragment_idx(), 0);
    assert_eq!(opus_audio_state.current_segment_idx(), 1);
    assert_eq!(opus_audio_state.discontinuity_sequence(), 0);
    assert_eq!(opus_audio_state.track_count(), 1);
    assert_eq!(opus_audio_state.track_duration(0), Some(48440));
    assert_eq!(opus_audio_state.track_timescale(0), Some(48000));
    assert_eq!(opus_audio_state.longest_segment(), 48440.0 / 48000.0);

    let aac_audio_state: HashMap<String, String> = redis
        .hgetall(format!("transcoder:{}:{}:state", req_id, aac_audio_id))
        .await
        .unwrap();
    let aac_audio_state = PlaylistState::from(aac_audio_state);

    assert_eq!(aac_audio_state.current_fragment_idx(), 0);
    assert_eq!(aac_audio_state.current_segment_idx(), 1);
    assert_eq!(aac_audio_state.discontinuity_sequence(), 0);
    assert_eq!(aac_audio_state.track_count(), 1);
    assert_eq!(aac_audio_state.track_duration(0), Some(49152));
    assert_eq!(aac_audio_state.track_timescale(0), Some(48000));
    assert_eq!(aac_audio_state.longest_segment(), 49152.0 / 48000.0);

    {
        let segment_state: HashMap<String, String> = redis
            .hgetall(format!("transcoder:{}:{}:0:state", req_id, source_video_id))
            .await
            .unwrap();
        let segment_state = SegmentState::from(segment_state);

        assert_eq!(segment_state.fragments().len(), 4);
        assert!(segment_state.fragments()[0].keyframe);
    }

    {
        let segment_state: HashMap<String, String> = redis
            .hgetall(format!("transcoder:{}:{}:0:state", req_id, aac_audio_id))
            .await
            .unwrap();
        let segment_state = SegmentState::from(segment_state);

        assert_eq!(segment_state.fragments().len(), 4);
        assert!(segment_state.fragments().iter().all(|f| f.keyframe));
    }

    {
        let segment_state: HashMap<String, String> = redis
            .hgetall(format!("transcoder:{}:{}:0:state", req_id, opus_audio_id))
            .await
            .unwrap();
        let segment_state = SegmentState::from(segment_state);

        assert_eq!(segment_state.fragments().len(), 4);
        assert!(segment_state.fragments().iter().all(|f| f.keyframe));
    }

    {
        let segment_state: HashMap<String, String> = redis
            .hgetall(format!("transcoder:{}:{}:0:state", req_id, video_id_360p))
            .await
            .unwrap();
        let segment_state = SegmentState::from(segment_state);

        assert_eq!(segment_state.fragments().len(), 4);
        assert!(segment_state.fragments()[0].keyframe);
    }

    tracing::info!("done");
}
