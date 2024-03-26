use std::fmt::{Debug, Formatter};
use std::sync::Arc;

use error::RouterError;
use route::RouteHandler;

use self::builder::RouterBuilder;
use self::middleware::{Middleware, NextFn, NextFut};
use self::types::{RouteInfo, RouteParams};

pub mod builder;
pub mod compat;
pub mod error;
pub mod ext;
pub mod middleware;
pub mod route;
pub mod types;

pub struct Router<I, O, E> {
	routes: Vec<RouteHandler<I, O, E>>,
	middlewares: Vec<Arc<dyn Middleware<I, O, E>>>,
	tree: path_tree::PathTree<RouteInfo>,
}

impl<I: Send + 'static, O: Send + 'static, E: Send + 'static> Router<I, O, E> {
	pub fn builder() -> RouterBuilder<I, O, E> {
		RouterBuilder::new()
	}

	pub async fn handle(&self, mut req: hyper::Request<I>) -> Result<hyper::Response<O>, RouterError<E>> {
		let path = format!("/{}{}", req.method().as_str(), req.uri().path());
		let (info, path) = self.tree.find(&path).ok_or(RouterError::NotFound)?;

		req.extensions_mut().insert(RouteParams(
			path.params_iter().map(|(k, v)| (k.to_owned(), v.to_owned())).collect(),
		));

		let handler = self.routes[info.route].clone();

		let next = Box::new(move |req| Box::pin(async move { handler(req).await }) as NextFut<O, E>) as NextFn<I, O, E>;

		info.middleware
			.iter()
			.rev()
			.map(|i| self.middlewares[*i].clone())
			.fold(next, |next, middleware| {
				Box::new(move |req| Box::pin(async move { middleware.handle(req, next).await }) as NextFut<O, E>)
			})(req)
		.await
		.map_err(RouterError::Unhandled)
	}
}

impl<I, O, E> Debug for Router<I, O, E> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Router").field("tree", &self.tree).finish()
	}
}
