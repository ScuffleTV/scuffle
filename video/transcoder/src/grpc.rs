use crate::global::TranscoderGlobal;

use std::sync::Arc;
use tonic::transport::server::Router;

pub fn add_routes<G: TranscoderGlobal, L>(_: &Arc<G>, router: Router<L>) -> Router<L> {
    router
}
