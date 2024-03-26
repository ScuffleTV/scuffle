use std::fmt::{Debug, Formatter};
use std::pin::Pin;
use std::sync::Arc;

use futures::Future;

use super::builder::RouterBuilder;

pub(crate) enum RouterItem<I, O, E> {
	Route(Route<I, O, E>),
	Router(RouterBuilder<I, O, E>),
}

impl<I, O, E> Debug for RouterItem<I, O, E> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			RouterItem::Route(route) => write!(f, "Route({route:?})"),
			RouterItem::Router(builder) => write!(f, "Router({builder:?})"),
		}
	}
}

pub(crate) type RouteHandler<I, O, E> = Arc<
	dyn Fn(hyper::Request<I>) -> Pin<Box<dyn Future<Output = Result<hyper::Response<O>, E>> + Send + 'static>>
		+ Send
		+ Sync
		+ 'static,
>;

pub(crate) struct Route<I, O, E> {
	pub method: Option<hyper::Method>,
	pub handler: RouteHandler<I, O, E>,
}

impl<I, O, E> Debug for Route<I, O, E> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Route").field("method", &self.method).finish()
	}
}
