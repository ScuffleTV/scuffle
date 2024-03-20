use std::fmt::{Debug, Formatter};
use std::sync::Arc;

use super::middleware::{Middleware, NextFn};
use super::route::{Route, RouterItem};
use super::types::RouteInfo;
use super::Router;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Copy)]
enum MiddlewareKind {
	Data,
	Error,
	Generic,
}

pub struct RouterBuilder<I, O, E> {
	tree: Vec<(&'static str, RouterItem<I, O, E>)>,
	middlewares: Vec<(Arc<dyn Middleware<I, O, E>>, MiddlewareKind)>,
}

impl<I, O, E> Debug for RouterBuilder<I, O, E> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("RouterBuilder").field("tree", &self.tree).finish()
	}
}

impl<I: 'static + Send, O: Send + 'static, E: Send + 'static> Default for RouterBuilder<I, O, E> {
	fn default() -> Self {
		Self::new()
	}
}

impl<I: Send + 'static + Send, O: Send + 'static, E: Send + 'static> RouterBuilder<I, O, E> {
	pub fn new() -> Self {
		Self {
			tree: Vec::new(),
			middlewares: Vec::new(),
		}
	}

	pub fn middleware(mut self, middleware: impl Middleware<I, O, E> + 'static) -> Self {
		self.middlewares.push((Arc::new(middleware), MiddlewareKind::Generic));
		self
	}

	pub fn data<T: Clone + Send + Sync + 'static>(mut self, data: T) -> Self {
		self.middlewares.push((
			Arc::new(move |mut req: hyper::Request<I>, next: NextFn<I, O, E>| {
				let data = data.clone();
				req.extensions_mut().insert(data);
				next(req)
			}),
			MiddlewareKind::Data,
		));

		self
	}

	pub fn error_handler<F: std::future::Future<Output = hyper::Response<O>> + Send + 'static>(
		mut self,
		handler: impl Fn(hyper::Request<()>, E) -> F + Send + Sync + 'static,
	) -> Self {
		let handler = Arc::new(handler);
		self.middlewares.push((
			Arc::new(move |req: hyper::Request<I>, next: NextFn<I, O, E>| {
				let handler = handler.clone();
				async move {
					let (parts, body) = req.into_parts();

					match next(hyper::Request::from_parts(parts.clone(), body)).await {
						Ok(res) => Ok(res),
						Err(err) => Ok(handler(hyper::Request::from_parts(parts, ()), err).await),
					}
				}
			}),
			MiddlewareKind::Error,
		));

		self
	}

	pub fn get<F: std::future::Future<Output = Result<hyper::Response<O>, E>> + Send + 'static>(
		self,
		path: &'static str,
		handler: impl Fn(hyper::Request<I>) -> F + Send + Sync + 'static,
	) -> Self {
		self.add_route(Some(hyper::Method::GET), path, handler)
	}

	pub fn post<F: std::future::Future<Output = Result<hyper::Response<O>, E>> + Send + 'static>(
		self,
		path: &'static str,
		handler: impl Fn(hyper::Request<I>) -> F + Send + Sync + 'static,
	) -> Self {
		self.add_route(Some(hyper::Method::POST), path, handler)
	}

	pub fn put<F: std::future::Future<Output = Result<hyper::Response<O>, E>> + Send + 'static>(
		self,
		path: &'static str,
		handler: impl Fn(hyper::Request<I>) -> F + Send + Sync + 'static,
	) -> Self {
		self.add_route(Some(hyper::Method::PUT), path, handler)
	}

	pub fn delete<F: std::future::Future<Output = Result<hyper::Response<O>, E>> + Send + 'static>(
		self,
		path: &'static str,
		handler: impl Fn(hyper::Request<I>) -> F + Send + Sync + 'static,
	) -> Self {
		self.add_route(Some(hyper::Method::DELETE), path, handler)
	}

	pub fn patch<F: std::future::Future<Output = Result<hyper::Response<O>, E>> + Send + 'static>(
		self,
		path: &'static str,
		handler: impl Fn(hyper::Request<I>) -> F + Send + Sync + 'static,
	) -> Self {
		self.add_route(Some(hyper::Method::PATCH), path, handler)
	}

	pub fn head<F: std::future::Future<Output = Result<hyper::Response<O>, E>> + Send + 'static>(
		self,
		path: &'static str,
		handler: impl Fn(hyper::Request<I>) -> F + Send + Sync + 'static,
	) -> Self {
		self.add_route(Some(hyper::Method::HEAD), path, handler)
	}

	pub fn options<F: std::future::Future<Output = Result<hyper::Response<O>, E>> + Send + 'static>(
		self,
		path: &'static str,
		handler: impl Fn(hyper::Request<I>) -> F + Send + Sync + 'static,
	) -> Self {
		self.add_route(Some(hyper::Method::OPTIONS), path, handler)
	}

	pub fn trace<F: std::future::Future<Output = Result<hyper::Response<O>, E>> + Send + 'static>(
		self,
		path: &'static str,
		handler: impl Fn(hyper::Request<I>) -> F + Send + Sync + 'static,
	) -> Self {
		self.add_route(Some(hyper::Method::TRACE), path, handler)
	}

	pub fn connect<F: std::future::Future<Output = Result<hyper::Response<O>, E>> + Send + 'static>(
		self,
		path: &'static str,
		handler: impl Fn(hyper::Request<I>) -> F + Send + Sync + 'static,
	) -> Self {
		self.add_route(Some(hyper::Method::CONNECT), path, handler)
	}

	pub fn any<F: std::future::Future<Output = Result<hyper::Response<O>, E>> + Send + 'static>(
		self,
		path: &'static str,
		handler: impl Fn(hyper::Request<I>) -> F + Send + Sync + 'static,
	) -> Self {
		self.add_route(None, path, handler)
	}

	pub fn add_route<F: std::future::Future<Output = Result<hyper::Response<O>, E>> + Send + 'static>(
		mut self,
		method: Option<hyper::Method>,
		path: &'static str,
		handler: impl Fn(hyper::Request<I>) -> F + Send + Sync + 'static,
	) -> Self {
		self.tree.push((
			path,
			RouterItem::Route(Route {
				method,
				handler: Arc::new(move |req| Box::pin(handler(req))),
			}),
		));
		self
	}

	pub fn scope(mut self, path: &'static str, router: RouterBuilder<I, O, E>) -> Self {
		self.tree.push((path, RouterItem::Router(router)));
		self
	}

	pub fn not_found<F: std::future::Future<Output = Result<hyper::Response<O>, E>> + Send + 'static>(
		self,
		handler: impl Fn(hyper::Request<I>) -> F + Send + Sync + 'static,
	) -> Self {
		self.add_route(None, "/*", handler)
	}

	fn build_scoped(mut self, parent_path: &str, target: &mut Router<I, O, E>, middlewares: &[usize]) {
		self.middlewares.sort_by_key(|(_, kind)| *kind);

		let middleware_idxs = middlewares
			.iter()
			.copied()
			.chain(self.middlewares.into_iter().map(|(handler, _)| {
				target.middlewares.push(handler);
				target.middlewares.len() - 1
			}))
			.collect::<Vec<_>>();

		for (path, item) in self.tree.drain(..) {
			match item {
				RouterItem::Route(route) => {
					target.routes.push(route.handler);

					let info = RouteInfo {
						route: target.routes.len() - 1,
						middleware: middleware_idxs.clone(),
					};

					let method = if let Some(method) = &route.method {
						method.as_str()
					} else {
						"*"
					};

					let parent_path = parent_path.trim_matches('/');
					let path = path.trim_matches('/');

					let full_path = format!(
						"/{method}/{}{}{}",
						parent_path,
						if parent_path.is_empty() || path.is_empty() { "" } else { "/" },
						path
					);

					tracing::debug!(parent_path, path, full_path, "adding route");

					let _ = target.tree.insert(&full_path, info);
				}
				RouterItem::Router(router) => {
					let parent_path = parent_path.trim_matches('/');
					let path = path.trim_matches('/');
					router.build_scoped(
						&format!(
							"{parent_path}{}{path}",
							if parent_path.is_empty() || path.is_empty() { "" } else { "/" }
						),
						target,
						&middleware_idxs,
					);
				}
			}
		}
	}

	pub fn build(self) -> Router<I, O, E> {
		let mut router = Router {
			routes: Vec::new(),
			middlewares: Vec::new(),
			tree: path_tree::PathTree::new(),
		};

		self.build_scoped("", &mut router, &[]);

		router
	}
}
