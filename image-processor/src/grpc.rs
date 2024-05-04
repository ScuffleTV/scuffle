use std::sync::Arc;

use tonic::transport::server::Router;

use crate::global::ImageProcessorGlobal;

pub fn add_routes<L>(_: &Arc<impl ImageProcessorGlobal>, router: Router<L>) -> Router<L> {
	router
}
