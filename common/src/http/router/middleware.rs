use std::fmt::{Debug, Formatter};

use super::types::{BoxFunction, BoxFuture};

pub struct PreMiddlewareHandler<E>(pub(crate) BoxFunction<hyper::Request<()>, BoxFuture<Result<hyper::Request<()>, E>>>);

impl<E> Debug for PreMiddlewareHandler<E> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "PreMiddlewareHandler(..)")
	}
}

pub struct PostMiddlewareHandler<O, E>(
	#[allow(clippy::type_complexity)]
	pub(crate)  BoxFunction<(hyper::Response<O>, hyper::Request<()>), BoxFuture<Result<hyper::Response<O>, E>>>,
);

impl<O, E> Debug for PostMiddlewareHandler<O, E> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "PostMiddlewareHandler(..)")
	}
}

pub enum Middleware<O, E> {
	Pre(PreMiddlewareHandler<E>),
	Post(PostMiddlewareHandler<O, E>),
}

impl<O, E> Debug for Middleware<O, E> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Middleware::Pre(_) => write!(f, "Pre(..)"),
			Middleware::Post(_) => write!(f, "Post(..)"),
		}
	}
}

impl<O: 'static, E: 'static> Middleware<O, E> {
	pub fn pre<F: std::future::Future<Output = Result<hyper::Request<()>, E>> + Send + Sync + 'static>(
		handler: impl Fn(hyper::Request<()>) -> F + Send + Sync + 'static,
	) -> Self {
		Self::Pre(PreMiddlewareHandler(Box::new(move |req| Box::pin(handler(req)))))
	}

	pub fn post<F: std::future::Future<Output = Result<hyper::Response<O>, E>> + Send + Sync + 'static>(
		handler: impl Fn(hyper::Response<O>) -> F + Send + Sync + 'static,
	) -> Self {
		Self::Post(PostMiddlewareHandler(Box::new(move |(res, _)| Box::pin(handler(res)))))
	}

	pub fn post_with_req<F: std::future::Future<Output = Result<hyper::Response<O>, E>> + Send + Sync + 'static>(
		handler: impl Fn(hyper::Response<O>, hyper::Request<()>) -> F + Send + Sync + 'static,
	) -> Self {
		Self::Post(PostMiddlewareHandler(Box::new(move |(res, req)| Box::pin(handler(res, req)))))
	}
}
