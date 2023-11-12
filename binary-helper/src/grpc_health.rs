use std::{
    pin::Pin,
    sync::{Arc, Weak},
};

use async_stream::try_stream;
use futures_util::{Future, Stream};
use tonic::{async_trait, Request, Response, Status};

use pb::grpc::health::v1::{
    health_check_response::ServingStatus, health_server, HealthCheckRequest, HealthCheckResponse,
};

pub struct HealthServer<
    G,
    F: Future<Output = bool> + Send + Sync + 'static,
    H: Fn(Arc<G>, HealthCheckRequest) -> F + Send + Sync + 'static,
> {
    global: Weak<G>,
    health_check: Arc<H>,
}

impl<
        G: Send + Sync + 'static,
        F: Future<Output = bool> + Send + Sync + 'static,
        H: Fn(Arc<G>, HealthCheckRequest) -> F + Send + Sync + 'static,
    > HealthServer<G, F, H>
{
    pub fn new(global: &Arc<G>, health_check: H) -> health_server::HealthServer<Self> {
        health_server::HealthServer::new(Self {
            global: Arc::downgrade(global),
            health_check: Arc::new(health_check),
        })
    }
}

type Result<T> = std::result::Result<T, Status>;

#[async_trait]
impl<
        G: Send + Sync + 'static,
        F: Future<Output = bool> + Send + Sync + 'static,
        H: Fn(Arc<G>, HealthCheckRequest) -> F + Send + Sync + 'static,
    > health_server::Health for HealthServer<G, F, H>
{
    type WatchStream = Pin<Box<dyn Stream<Item = Result<HealthCheckResponse>> + Send>>;

    async fn check(
        &self,
        req: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>> {
        let global = self
            .global
            .upgrade()
            .ok_or_else(|| Status::internal("global state dropped"))?;

        let serving = (self.health_check)(global, req.into_inner()).await;

        Ok(Response::new(HealthCheckResponse {
            status: if serving {
                ServingStatus::Serving.into()
            } else {
                ServingStatus::NotServing.into()
            },
        }))
    }

    async fn watch(&self, req: Request<HealthCheckRequest>) -> Result<Response<Self::WatchStream>> {
        let global = self.global.clone();
        let health_check = self.health_check.clone();
        let req = req.into_inner();

        let output = try_stream!({
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                let global = match global.upgrade() {
                    Some(global) => global,
                    None => {
                        yield HealthCheckResponse {
                            status: ServingStatus::NotServing.into(),
                        };
                        return;
                    }
                };

                let serving = (health_check)(global, req.clone()).await;

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
