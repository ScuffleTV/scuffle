// TODO: This is the test stub for the transcoder service. It is not yet implemented.
#![allow(unused_imports)]
#![allow(dead_code)]

use std::{net::SocketAddr, pin::Pin, sync::Arc};

use async_trait::async_trait;
use chrono::Utc;
use futures_util::Stream;
use lapin::BasicProperties;
use prost::Message;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response};
use uuid::Uuid;

use crate::{
    config::{AppConfig, RmqConfig},
    global::{self, GlobalState},
    pb::scuffle::{
        events::{self, transcoder_message},
        video::{
            ingest_server::{Ingest, IngestServer},
            ShutdownStreamRequest, ShutdownStreamResponse, TranscoderEventRequest,
            TranscoderEventResponse, WatchStreamRequest, WatchStreamResponse,
        },
    },
    transcoder,
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

// #[tokio::test]
// async fn test_transcode() {
//     let port = portpicker::pick_unused_port().unwrap();

//     let (global, handler) = crate::tests::global::mock_global_state(AppConfig {
//         rmq: RmqConfig {
//             transcoder_queue: Uuid::new_v4().to_string(),
//             uri: "".to_string(),
//         },
//         ..Default::default()
//     })
//     .await;

//     global::init_rmq(&global, false).await;

//     let addr = SocketAddr::from(([127, 0, 0, 1], port));

//     let mut rx = setup_ingest_server(global.clone(), addr);

//     let transcoder_run_handle = tokio::spawn(transcoder::run(global.clone()));

//     let channel = global.rmq.aquire().await.unwrap();

//     let req_id = Uuid::new_v4();

//     channel
//         .basic_publish(
//             "",
//             &global.config.rmq.transcoder_queue,
//             lapin::options::BasicPublishOptions::default(),
//             events::TranscoderMessage {
//                 id: req_id.to_string(),
//                 timestamp: Utc::now().timestamp() as u64,
//                 data: Some(transcoder_message::Data::NewStream(
//                     events::TranscoderMessageNewStream {
//                         request_id: req_id.to_string(),
//                         stream_id: req_id.to_string(),
//                         ingest_address: addr.to_string(),
//                         variants: None,
//                     },
//                 )),
//             }
//             .encode_to_vec()
//             .as_slice(),
//             BasicProperties::default()
//                 .with_message_id(req_id.to_string().into())
//                 .with_content_type("application/octet-stream".into())
//                 .with_expiration("60000".into()),
//         )
//         .await
//         .unwrap();

//     let watch_stream_req = match rx.recv().await.unwrap() {
//         IngestRequest::WatchStream { request, tx } => {
//             assert_eq!(request.stream_id, req_id.to_string());
//             assert_eq!(request.request_id, req_id.to_string());

//             tx
//         }
//         _ => panic!("unexpected request"),
//     };

//     drop(global);
//     handler.cancel().await;
// }
