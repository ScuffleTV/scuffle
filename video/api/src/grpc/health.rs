use crate::global::GlobalState;
use std::{
    pin::Pin,
    sync::{Arc, Weak},
};

use async_stream::try_stream;
use futures_util::Stream;
use tonic::{async_trait, Request, Response, Status};

use pb::grpc::health::v1::{
    health_check_response::ServingStatus,
    health_server::{Health, HealthServer as HealthService},
    HealthCheckRequest, HealthCheckResponse,
};

pub struct HealthServer {
    global: Weak<GlobalState>,
}

impl HealthServer {
    pub fn new(global: &Arc<GlobalState>) -> HealthService<Self> {
        HealthService::new(Self {
            global: Arc::downgrade(global),
        })
    }
}

type Result<T> = std::result::Result<T, Status>;

#[async_trait]
impl Health for HealthServer {
    type WatchStream = Pin<Box<dyn Stream<Item = Result<HealthCheckResponse>> + Send>>;

    async fn check(&self, _: Request<HealthCheckRequest>) -> Result<Response<HealthCheckResponse>> {
        let serving = self
            .global
            .upgrade()
            .map(|g| !g.ctx.is_done())
            .unwrap_or_default();

        Ok(Response::new(HealthCheckResponse {
            status: if serving {
                ServingStatus::Serving.into()
            } else {
                ServingStatus::NotServing.into()
            },
        }))
    }

    async fn watch(&self, _: Request<HealthCheckRequest>) -> Result<Response<Self::WatchStream>> {
        let global = self.global.clone();

        let output = try_stream!({
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                let serving = global
                    .upgrade()
                    .map(|g| !g.ctx.is_done())
                    .unwrap_or_default();

                yield HealthCheckResponse {
                    status: if serving {
                        ServingStatus::Serving.into()
                    } else {
                        ServingStatus::NotServing.into()
                    },
                };
            }
        });

        Ok(Response::new(Box::pin(output)))
    }
}
