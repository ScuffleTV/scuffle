use crate::{
    connection_manager::{GrpcRequest, WatchStreamEvent},
    global::GlobalState,
    pb::scuffle::video::{
        ingest_server, transcoder_event_request, watch_stream_response, ShutdownStreamRequest,
        ShutdownStreamResponse, TranscoderEventRequest, TranscoderEventResponse,
        WatchStreamRequest, WatchStreamResponse,
    },
};
use std::{
    pin::Pin,
    sync::{Arc, Weak},
};

use async_stream::try_stream;
use futures::Stream;
use tokio::sync::mpsc;
use tonic::{async_trait, Request, Response, Status};
use uuid::Uuid;

pub struct IngestServer {
    global: Weak<GlobalState>,
}

impl IngestServer {
    pub fn new(global: &Arc<GlobalState>) -> Self {
        Self {
            global: Arc::downgrade(global),
        }
    }
}

type Result<T> = std::result::Result<T, Status>;

#[async_trait]
impl ingest_server::Ingest for IngestServer {
    type WatchStreamStream =
        Pin<Box<dyn Stream<Item = Result<WatchStreamResponse>> + 'static + Send>>;

    async fn watch_stream(
        &self,
        request: Request<WatchStreamRequest>,
    ) -> Result<Response<Self::WatchStreamStream>> {
        let global = self
            .global
            .upgrade()
            .ok_or_else(|| Status::internal("Global state is gone"))?;

        let request = request.into_inner();

        let request_id = Uuid::parse_str(&request.request_id)
            .map_err(|_| Status::invalid_argument("Invalid request ID"))?;
        let stream_id = Uuid::parse_str(&request.stream_id)
            .map_err(|_| Status::invalid_argument("Invalid stream ID"))?;

        let (channel_tx, mut channel_rx) = mpsc::channel(256);

        let request = GrpcRequest::WatchStream {
            id: request_id,
            channel: channel_tx,
        };

        if !global
            .connection_manager
            .submit_request(stream_id, request)
            .await
        {
            return Err(Status::not_found("Stream not found"));
        }

        let output = try_stream! {
            while let Some(event) = channel_rx.recv().await {
                let event = match event {
                    WatchStreamEvent::InitSegment(data) => {
                        WatchStreamResponse {
                            data: Some(watch_stream_response::Data::InitSegment(data)),
                        }
                    },
                    WatchStreamEvent::MediaSegment(ms) => {
                        WatchStreamResponse {
                            data: Some(watch_stream_response::Data::MediaSegment(
                                watch_stream_response::MediaSegment {
                                    data: ms.data,
                                    keyframe: ms.keyframe,
                                    timestamp: ms.timestamp,
                                    data_type: match ms.ty {
                                        transmuxer::MediaType::Audio => watch_stream_response::media_segment::DataType::Audio.into(),
                                        transmuxer::MediaType::Video => watch_stream_response::media_segment::DataType::Video.into(),
                                    }
                                }
                            )),
                        }
                    }
                    WatchStreamEvent::ShuttingDown(stream_shutdown) => {
                        WatchStreamResponse {
                            data: Some(watch_stream_response::Data::ShuttingDown(stream_shutdown)),
                        }
                    }
                };

                yield event;
            }
        };

        Ok(Response::new(Box::pin(output)))
    }

    async fn transcoder_event(
        &self,
        request: Request<TranscoderEventRequest>,
    ) -> Result<Response<TranscoderEventResponse>> {
        let global = self
            .global
            .upgrade()
            .ok_or_else(|| Status::internal("Global state is gone"))?;

        let request = request.into_inner();

        let request_id = Uuid::parse_str(&request.request_id)
            .map_err(|_| Status::invalid_argument("Invalid request ID"))?;
        let stream_id = Uuid::parse_str(&request.stream_id)
            .map_err(|_| Status::invalid_argument("Invalid stream ID"))?;

        let request = match request.event {
            Some(transcoder_event_request::Event::Started(_)) => {
                GrpcRequest::TranscoderStarted { id: request_id }
            }
            Some(transcoder_event_request::Event::ShuttingDown(_)) => {
                GrpcRequest::TranscoderShuttingDown { id: request_id }
            }
            Some(transcoder_event_request::Event::Error(error)) => GrpcRequest::TranscoderError {
                id: request_id,
                message: error.message,
                fatal: error.fatal,
            },
            None => return Err(Status::invalid_argument("Invalid event")),
        };

        if !global
            .connection_manager
            .submit_request(stream_id, request)
            .await
        {
            return Err(Status::not_found("Stream not found"));
        }

        Ok(Response::new(TranscoderEventResponse {}))
    }

    async fn shutdown_stream(
        &self,
        request: Request<ShutdownStreamRequest>,
    ) -> Result<Response<ShutdownStreamResponse>> {
        let global = self
            .global
            .upgrade()
            .ok_or_else(|| Status::internal("Global state is gone"))?;

        let request = request.into_inner();

        let stream_id = Uuid::parse_str(&request.stream_id)
            .map_err(|_| Status::invalid_argument("Invalid stream ID"))?;

        let request = GrpcRequest::ShutdownStream;

        if !global
            .connection_manager
            .submit_request(stream_id, request)
            .await
        {
            return Err(Status::not_found("Stream not found"));
        }

        Ok(Response::new(ShutdownStreamResponse {}))
    }
}
