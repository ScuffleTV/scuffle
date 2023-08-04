use crate::global::GlobalState;
use std::{
    pin::Pin,
    sync::{Arc, Weak},
};

use async_stream::try_stream;
use futures_util::Stream;
use tonic::{async_trait, Request, Response, Status};

use pb::grpc::health::v1::{
    health_check_response::ServingStatus, health_server, HealthCheckRequest, HealthCheckResponse,
};

#[derive(Clone)]
pub struct HealthServer {
    global: Weak<GlobalState>,
}

impl HealthServer {
    pub fn new(global: &Arc<GlobalState>) -> health_server::HealthServer<Self> {
        health_server::HealthServer::new(Self {
            global: Arc::downgrade(global),
        })
    }

    async fn health(&self) -> Result<HealthCheckResponse> {
        let serving = self
            .global
            .upgrade()
            .map(|g| !g.ctx.is_done())
            .unwrap_or_default();

        Ok(HealthCheckResponse {
            status: if serving {
                ServingStatus::Serving.into()
            } else {
                ServingStatus::NotServing.into()
            },
        })
    }
}

type Result<T> = std::result::Result<T, Status>;

#[async_trait]
impl health_server::Health for HealthServer {
    type WatchStream = Pin<Box<dyn Stream<Item = Result<HealthCheckResponse>> + Send>>;

    async fn check(&self, _: Request<HealthCheckRequest>) -> Result<Response<HealthCheckResponse>> {
        Ok(Response::new(self.health().await?))
    }

    async fn watch(&self, _: Request<HealthCheckRequest>) -> Result<Response<Self::WatchStream>> {
        let this = self.clone();

        let output = try_stream!({
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                yield this.health().await?;
            }
        });

        Ok(Response::new(Box::pin(output)))
    }
}
