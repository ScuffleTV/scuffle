use bytes::Bytes;
use common::grpc::make_channel;
use common::prelude::FutureTimeout;
use std::time::Duration;
use transmuxer::{MediaSegment, MediaType};
use uuid::Uuid;

use crate::{
    config::{AppConfig, GrpcConfig},
    connection_manager::{GrpcRequest, WatchStreamEvent},
    grpc::run,
    pb::scuffle::video::{transcoder_event_request, watch_stream_response, TranscoderEventRequest},
    tests::global::mock_global_state,
};

#[tokio::test]
async fn test_grpc_ingest_transcoder_event() {
    let port = portpicker::pick_unused_port().expect("failed to pick port");
    let (global, handler) = mock_global_state(AppConfig {
        grpc: GrpcConfig {
            bind_address: format!("0.0.0.0:{}", port).parse().unwrap(),
            ..Default::default()
        },
        ..Default::default()
    })
    .await;

    let handle = tokio::spawn(run(global.clone()));

    let channel = make_channel(
        vec![format!("http://localhost:{}", port)],
        Duration::from_secs(0),
        None,
    )
    .unwrap();

    let stream_id = Uuid::new_v4();

    let (tx, mut rx) = tokio::sync::mpsc::channel(1);

    global
        .connection_manager
        .register_stream(stream_id, Uuid::new_v4(), tx)
        .await;

    let mut client = crate::pb::scuffle::video::ingest_client::IngestClient::new(channel);

    let request_id = Uuid::new_v4();

    client
        .transcoder_event(TranscoderEventRequest {
            stream_id: stream_id.to_string(),
            request_id: request_id.to_string(),
            event: Some(transcoder_event_request::Event::Started(true)),
        })
        .await
        .unwrap();

    let event = rx
        .recv()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to receive event")
        .expect("failed to receive event");

    match event {
        GrpcRequest::Started { id } => {
            assert_eq!(id, request_id);
        }
        _ => panic!("wrong request"),
    }

    client
        .transcoder_event(TranscoderEventRequest {
            stream_id: stream_id.to_string(),
            request_id: request_id.to_string(),
            event: Some(transcoder_event_request::Event::ShuttingDown(true)),
        })
        .await
        .unwrap();

    let event = rx
        .recv()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to receive event")
        .expect("failed to receive event");

    match event {
        GrpcRequest::ShuttingDown { id } => {
            assert_eq!(id, request_id);
        }
        _ => panic!("wrong request"),
    }

    client
        .transcoder_event(TranscoderEventRequest {
            stream_id: stream_id.to_string(),
            request_id: request_id.to_string(),
            event: Some(transcoder_event_request::Event::Error(
                transcoder_event_request::Error {
                    message: "test".to_string(),
                    fatal: false,
                },
            )),
        })
        .await
        .unwrap();

    let event = rx
        .recv()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to receive event")
        .expect("failed to receive event");

    match event {
        GrpcRequest::Error {
            id,
            message,
            fatal: _,
        } => {
            assert_eq!(id, request_id);
            assert_eq!(message, "test");
        }
        _ => panic!("wrong request"),
    }

    drop(global);

    handler
        .cancel()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel context");

    handle
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel grpc")
        .expect("grpc failed")
        .expect("grpc failed");
}

#[tokio::test]
async fn test_grpc_ingest_watch_stream() {
    let port = portpicker::pick_unused_port().expect("failed to pick port");
    let (global, handler) = mock_global_state(AppConfig {
        grpc: GrpcConfig {
            bind_address: format!("0.0.0.0:{}", port).parse().unwrap(),
            ..Default::default()
        },
        ..Default::default()
    })
    .await;

    let handle = tokio::spawn(run(global.clone()));

    let channel = make_channel(
        vec![format!("http://localhost:{}", port)],
        Duration::from_secs(0),
        None,
    )
    .unwrap();

    let stream_id = Uuid::new_v4();

    let (tx, mut rx) = tokio::sync::mpsc::channel(1);

    global
        .connection_manager
        .register_stream(stream_id, Uuid::new_v4(), tx)
        .await;

    let mut client = crate::pb::scuffle::video::ingest_client::IngestClient::new(channel);

    let request_id = Uuid::new_v4();

    let mut revc_stream = client
        .watch_stream(crate::pb::scuffle::video::WatchStreamRequest {
            stream_id: stream_id.to_string(),
            request_id: request_id.to_string(),
        })
        .await
        .unwrap()
        .into_inner();

    let event = rx
        .recv()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to receive event")
        .expect("failed to receive event");

    let ch = match event {
        GrpcRequest::WatchStream { id, channel } => {
            assert_eq!(id, request_id);
            channel
        }
        _ => panic!("wrong request"),
    };

    ch.send(WatchStreamEvent::InitSegment(Bytes::from_static(
        b"testing 123",
    )))
    .await
    .unwrap();
    let resp = revc_stream.message().await.unwrap().unwrap();
    assert_eq!(
        resp.data,
        Some(watch_stream_response::Data::InitSegment(
            b"testing 123".to_vec().into()
        ))
    );

    ch.send(WatchStreamEvent::MediaSegment(MediaSegment {
        data: Bytes::from_static(b"fragment"),
        keyframe: true,
        timestamp: 123,
        ty: MediaType::Video,
    }))
    .await
    .unwrap();
    let resp = revc_stream.message().await.unwrap().unwrap();
    assert_eq!(
        resp.data,
        Some(watch_stream_response::Data::MediaSegment(
            watch_stream_response::MediaSegment {
                data: b"fragment".to_vec().into(),
                keyframe: true,
                timestamp: 123,
                data_type: watch_stream_response::media_segment::DataType::Video as i32,
            }
        ))
    );

    ch.send(WatchStreamEvent::MediaSegment(MediaSegment {
        data: Bytes::from_static(b"fragment2"),
        keyframe: false,
        timestamp: 456,
        ty: MediaType::Audio,
    }))
    .await
    .unwrap();
    let resp = revc_stream.message().await.unwrap().unwrap();
    assert_eq!(
        resp.data,
        Some(watch_stream_response::Data::MediaSegment(
            watch_stream_response::MediaSegment {
                data: b"fragment2".to_vec().into(),
                keyframe: false,
                timestamp: 456,
                data_type: watch_stream_response::media_segment::DataType::Audio as i32,
            }
        ))
    );

    drop(global);

    handler
        .cancel()
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel context");

    handle
        .timeout(Duration::from_secs(1))
        .await
        .expect("failed to cancel grpc")
        .expect("grpc failed")
        .expect("grpc failed");
}
