use std::fmt::{Debug, Formatter};

use super::builder::RouterBuilder;
use super::types::{BoxFunction, BoxFuture};

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

pub(crate) struct RouteHandler<I, O, E>(pub BoxFunction<hyper::Request<I>, BoxFuture<Result<hyper::Response<O>, E>>>);

impl<I, O, E> Debug for RouteHandler<I, O, E> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "RouteHandler(..)")
	}
}

pub(crate) struct Route<I, O, E> {
	pub method: Option<hyper::Method>,
	pub handler: RouteHandler<I, O, E>,
}

impl<I, O, E> Debug for Route<I, O, E> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Route")
			.field("method", &self.method)
			.field("handler", &self.handler)
			.finish()
	}
}
