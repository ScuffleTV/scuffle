use common::grpc::{make_channel, TlsSettings};
use common::prelude::FutureTimeout;
use std::path::PathBuf;
use std::time::Duration;
use tonic::transport::{Certificate, Identity};

use crate::config::{AppConfig, GrpcConfig, TlsConfig};
use crate::grpc::run;
use crate::pb;
use crate::tests::global::mock_global_state;

#[tokio::test]
async fn test_grpc_tls_rsa() {
    let port = portpicker::pick_unused_port().expect("failed to pick port");
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/tests/grpc/certs");

    let (global, handler) = mock_global_state(AppConfig {
        grpc: GrpcConfig {
            bind_address: format!("0.0.0.0:{}", port).parse().unwrap(),
            tls: Some(TlsConfig {
                cert: dir.join("server.rsa.crt").to_str().unwrap().to_string(),
                ca_cert: dir.join("ca.rsa.crt").to_str().unwrap().to_string(),
                key: dir.join("server.rsa.key").to_str().unwrap().to_string(),
                domain: Some("localhost".to_string()),
            }),
        },
        ..Default::default()
    })
    .await;

    let ca_content =
        Certificate::from_pem(std::fs::read_to_string(dir.join("ca.rsa.crt")).unwrap());
    let client_cert = std::fs::read_to_string(dir.join("client.rsa.crt")).unwrap();
    let client_key = std::fs::read_to_string(dir.join("client.rsa.key")).unwrap();
    let client_identity = Identity::from_pem(client_cert, client_key);

    let channel = make_channel(
        vec![format!("https://localhost:{}", port)],
        Duration::from_secs(0),
        Some(TlsSettings {
            domain: "localhost".to_string(),
            ca_cert: ca_content,
            identity: client_identity,
        }),
    )
    .unwrap();

    let handle = tokio::spawn(async move {
        if let Err(e) = run(global).await {
            tracing::error!("grpc failed: {}", e);
            Err(e)
        } else {
            Ok(())
        }
    });

    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut client = pb::health::health_client::HealthClient::new(channel);

    let resp = client
        .check(pb::health::HealthCheckRequest::default())
        .await
        .unwrap();
    assert_eq!(
        resp.into_inner().status,
        pb::health::health_check_response::ServingStatus::Serving as i32
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
async fn test_grpc_tls_ec() {
    let port = portpicker::pick_unused_port().expect("failed to pick port");
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/tests/grpc/certs");

    let (global, handler) = mock_global_state(AppConfig {
        grpc: GrpcConfig {
            bind_address: format!("0.0.0.0:{}", port).parse().unwrap(),
            tls: Some(TlsConfig {
                cert: dir.join("server.ec.crt").to_str().unwrap().to_string(),
                ca_cert: dir.join("ca.ec.crt").to_str().unwrap().to_string(),
                key: dir.join("server.ec.key").to_str().unwrap().to_string(),
                domain: Some("localhost".to_string()),
            }),
        },
        ..Default::default()
    })
    .await;

    let ca_content = Certificate::from_pem(std::fs::read_to_string(dir.join("ca.ec.crt")).unwrap());
    let client_cert = std::fs::read_to_string(dir.join("client.ec.crt")).unwrap();
    let client_key = std::fs::read_to_string(dir.join("client.ec.key")).unwrap();
    let client_identity = Identity::from_pem(client_cert, client_key);

    let channel = make_channel(
        vec![format!("https://localhost:{}", port)],
        Duration::from_secs(0),
        Some(TlsSettings {
            domain: "localhost".to_string(),
            ca_cert: ca_content,
            identity: client_identity,
        }),
    )
    .unwrap();

    let handle = tokio::spawn(async move {
        if let Err(e) = run(global).await {
            tracing::error!("grpc failed: {}", e);
            Err(e)
        } else {
            Ok(())
        }
    });

    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut client = pb::health::health_client::HealthClient::new(channel);

    let resp = client
        .check(pb::health::HealthCheckRequest::default())
        .await
        .unwrap();
    assert_eq!(
        resp.into_inner().status,
        pb::health::health_check_response::ServingStatus::Serving as i32
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
