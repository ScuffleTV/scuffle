use crate::global::EdgeGlobal;

use std::sync::Arc;
use tonic::transport::server::Router;

pub fn add_routes<G: EdgeGlobal, L>(_: &Arc<G>, router: Router<L>) -> Router<L> {
    router
}
