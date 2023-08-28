use std::time::Duration;

use common::grpc::make_channel;
use common::prelude::FutureTimeout;
use ulid::Ulid;

use crate::config::{AppConfig, GrpcConfig, TranscoderConfig};
use crate::grpc::run;
use crate::tests::global::mock_global_state;

#[tokio::test]
async fn test_grpc_health_check() {
    let port = portpicker::pick_unused_port().expect("failed to pick port");
    let (global, handler) = mock_global_state(AppConfig {
        grpc: GrpcConfig {
            bind_address: format!("0.0.0.0:{}", port).parse().unwrap(),
            ..Default::default()
        },
        transcoder: TranscoderConfig {
            events_subject: Ulid::new().to_string(),
            media_ob_store: Ulid::new().to_string(),
            metadata_kv_store: Ulid::new().to_string(),
            transcoder_request_subject: Ulid::new().to_string(),
            ..Default::default()
        },
        ..Default::default()
    })
    .await;

    let handle = tokio::spawn(run(global));

    let channel = make_channel(
        vec![format!("http://localhost:{}", port)],
        Duration::from_secs(0),
        None,
    )
    .unwrap();

    let mut client = pb::grpc::health::v1::health_client::HealthClient::new(channel);
    let resp = client
        .check(pb::grpc::health::v1::HealthCheckRequest::default())
        .await
        .unwrap();
    assert_eq!(
        resp.into_inner().status,
        pb::grpc::health::v1::health_check_response::ServingStatus::Serving as i32
    );
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
async fn test_grpc_health_watch() {
    let port = portpicker::pick_unused_port().expect("failed to pick port");
    let (global, handler) = mock_global_state(AppConfig {
        grpc: GrpcConfig {
            bind_address: format!("0.0.0.0:{}", port).parse().unwrap(),
            ..Default::default()
        },
        transcoder: TranscoderConfig {
            events_subject: Ulid::new().to_string(),
            media_ob_store: Ulid::new().to_string(),
            metadata_kv_store: Ulid::new().to_string(),
            transcoder_request_subject: Ulid::new().to_string(),
            ..Default::default()
        },
        ..Default::default()
    })
    .await;

    let handle = tokio::spawn(run(global));
    let channel = make_channel(
        vec![format!("http://localhost:{}", port)],
        Duration::from_secs(0),
        None,
    )
    .unwrap();

    let mut client = pb::grpc::health::v1::health_client::HealthClient::new(channel);

    let resp = client
        .watch(pb::grpc::health::v1::HealthCheckRequest::default())
        .await
        .unwrap();

    let mut stream = resp.into_inner();
    let resp = stream.message().await.unwrap().unwrap();
    assert_eq!(
        resp.status,
        pb::grpc::health::v1::health_check_response::ServingStatus::Serving as i32
    );

    let cancel = handler.cancel();

    let resp = stream.message().await.unwrap().unwrap();
    assert_eq!(
        resp.status,
        pb::grpc::health::v1::health_check_response::ServingStatus::NotServing as i32
    );

    cancel
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
