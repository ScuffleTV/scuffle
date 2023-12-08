use std::sync::Arc;

use tonic::transport::server::Router;

use crate::global::ImageProcessorGlobal;

pub fn add_routes<G: ImageProcessorGlobal, L>(_: &Arc<G>, router: Router<L>) -> Router<L> {
	router
}
