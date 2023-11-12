use crate::global::ApiGlobal;

use std::sync::Arc;
use tonic::transport::server::Router;

pub fn add_routes<G: ApiGlobal, L>(_: &Arc<G>, router: Router<L>) -> Router<L> {
    router
}
