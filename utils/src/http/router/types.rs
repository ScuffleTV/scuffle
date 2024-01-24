use std::fmt::{Debug, Formatter};
use std::pin::Pin;

#[derive(Debug, Clone)]
pub struct RouteParams(pub Box<[(String, String)]>);

#[derive(Debug)]
pub(crate) struct RouteInfo {
	pub route: usize,
	pub pre_middleware: Vec<usize>,
	pub post_middleware: Vec<usize>,
	pub error_handler: Option<usize>,
}

pub(crate) type BoxFunction<I, O> = Box<dyn Fn(I) -> O + Send + Sync + 'static>;
pub(crate) type BoxFuture<O> = Pin<Box<dyn std::future::Future<Output = O> + Send + 'static>>;

pub(crate) struct ErrorHandler<O, E>(pub BoxFunction<(hyper::Request<()>, E), BoxFuture<hyper::Response<O>>>);

impl<O, E> Debug for ErrorHandler<O, E> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ErrorHandler(..)").finish()
	}
}
