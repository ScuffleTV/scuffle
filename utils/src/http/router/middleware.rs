use std::pin::Pin;

use futures::Future;
use tonic::async_trait;

mod cors;
mod response_headers;

pub use cors::{CorsMiddleware, CorsOptions};
pub use response_headers::{ResponseHeadersMiddleware, ResponseHeadersRequestExt};

use super::builder::RouterBuilder;

pub type NextFn<I, O, E> = Box<dyn FnOnce(hyper::Request<I>) -> NextFut<O, E> + Sync + Send + 'static>;
pub type NextFut<O, E> = Pin<Box<dyn Future<Output = Result<hyper::Response<O>, E>> + Send + 'static>>;

#[async_trait]
pub trait Middleware<I: Send, O: Send, E: Send>: Sync + Send + 'static {
	async fn handle(&self, req: hyper::Request<I>, next: NextFn<I, O, E>) -> Result<hyper::Response<O>, E>;

	fn extend(&self, builder: RouterBuilder<I, O, E>) -> RouterBuilder<I, O, E> {
		builder
	}
}

pub fn middleware_fn<I: Send + 'static, O: Send + 'static, E: Send + 'static, F, Fut>(f: F) -> impl Middleware<I, O, E>
where
	F: Fn(hyper::Request<I>, NextFn<I, O, E>) -> Fut + Sync + Send + 'static,
	Fut: std::future::Future<Output = Result<hyper::Response<O>, E>> + Send + 'static,
{
	f
}

#[async_trait]
impl<I: Send + 'static, O: Send + 'static, E: Send + 'static, F, Fut> Middleware<I, O, E> for F
where
	F: Fn(hyper::Request<I>, NextFn<I, O, E>) -> Fut + Sync + Send + 'static,
	Fut: std::future::Future<Output = Result<hyper::Response<O>, E>> + Send + 'static,
{
	async fn handle(&self, req: hyper::Request<I>, next: NextFn<I, O, E>) -> Result<hyper::Response<O>, E> {
		self(req, next).await
	}
}
