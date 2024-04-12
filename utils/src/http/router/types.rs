use std::pin::Pin;
use std::sync::Arc;

use futures::Future;

#[derive(Debug, Clone)]
pub struct RouteParams(pub Box<[(String, String)]>);

#[derive(Debug)]
pub(crate) struct RouteInfo {
	pub route: usize,
	pub error_handler: Option<usize>,
	pub middleware: Vec<usize>,
}

pub(crate) type ErrorHandler<O, E> = Arc<
	dyn Fn(hyper::Request<()>, E) -> Pin<Box<dyn Future<Output = http::Response<O>> + Send + 'static>>
		+ Send
		+ Sync
		+ 'static,
>;
