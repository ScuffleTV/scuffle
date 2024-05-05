use std::sync::Arc;

use anyhow::Context;
use scuffle_image_processor_proto::{CancelTaskRequest, CancelTaskResponse, Error, ErrorCode, ProcessImageRequest, ProcessImageResponse, input_path::InputPath};
use url::Url;

use crate::global::Global;

pub mod grpc;
pub mod http;

mod validation;

#[derive(Clone)]
struct ManagementServer {
    global: Arc<Global>,
}

impl ManagementServer {
    async fn process_image(
        &self,
        request: ProcessImageRequest,
    ) -> Result<ProcessImageResponse, Error> {
        let Some(task) = request.task.as_ref() else {
            return Err(Error {
                code: ErrorCode::InvalidInput as i32,
                message: "task is required".to_string(),
            });
        };

        let Some(input) = &task.input else {
            return Err(Error {
                code: ErrorCode::InvalidInput as i32,
                message: "input is required".to_string(),
            });
        };

        let Some(input_path) = input.path.as_ref().and_then(|path| path.input_path.as_ref()) else {
            return Err(Error {
                code: ErrorCode::InvalidInput as i32,
                message: "task.input.path is required".to_string(),
            });
        };

        

        if let Some(events) = &task.events {
            let queues = [
                (&events.on_success, "task.events.on_success"),
                (&events.on_fail, "task.events.on_fail"),
                (&events.on_start, "task.events.on_start"),
                (&events.on_cancel, "task.events.on_cancel"),
            ];

            for (queue, field) in queues {
                if let Some(queue) = queue {
                    if self.global.event_queue(&queue.name).is_none() {
                        return Err(Error {
                            code: ErrorCode::InvalidInput as i32,
                            message: format!("{field}.name: event queue not found"),
                        });
                    }
                }
            }            
        }

        // We need to do validation here.
        if let Some(image) = request.input_upload.as_ref() {

        }

        todo!()
    }

    async fn cancel_task(
        &self,
        request: CancelTaskRequest,
    ) -> Result<CancelTaskResponse, Error> {
        todo!()
    }
}


pub async fn start(global: Arc<Global>) -> anyhow::Result<()> {
    let server = ManagementServer {
        global,
    };

    let http = async {
        if server.global.config().management.http.enabled {
            server.run_http().await.context("http")
        } else {
            Ok(())
        }
    };
    let grpc = async {
        if server.global.config().management.grpc.enabled {
            server.run_grpc().await.context("grpc")
        } else {
            Ok(())
        }
    };

    futures::future::try_join(http, grpc).await.context("management")?;

    Ok(())
}

