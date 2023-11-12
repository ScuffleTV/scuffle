use crate::global::IngestGlobal;
use std::sync::Arc;
use tonic::transport::server::Router;

mod ingest;

pub fn add_routes<G: IngestGlobal, L>(global: &Arc<G>, router: Router<L>) -> Router<L> {
    router.add_service(ingest::IngestServer::new(global))
}
