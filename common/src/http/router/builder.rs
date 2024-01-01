use std::fmt::{Debug, Formatter};

use super::middleware::{Middleware, PostMiddlewareHandler, PreMiddlewareHandler};
use super::route::{Route, RouteHandler, RouterItem};
use super::types::{ErrorHandler, RouteInfo};
use super::Router;

pub struct RouterBuilder<I, O, E> {
	tree: Vec<(&'static str, RouterItem<I, O, E>)>,
	pre_middleware: Vec<PreMiddlewareHandler<E>>,
	post_middleware: Vec<PostMiddlewareHandler<O, E>>,
	error_handler: Option<ErrorHandler<O, E>>,
}

impl<I, O, E> Debug for RouterBuilder<I, O, E> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("RouterBuilder")
			.field("tree", &self.tree)
			.field("pre_middleware", &self.pre_middleware)
			.field("post_middleware", &self.post_middleware)
			.field("error_handler", &self.error_handler)
			.finish()
	}
}

impl<I: 'static, O: 'static, E: 'static> Default for RouterBuilder<I, O, E> {
	fn default() -> Self {
		Self::new()
	}
}

impl<I: 'static, O: 'static, E: 'static> RouterBuilder<I, O, E> {
	pub fn new() -> Self {
		Self {
			tree: Vec::new(),
			post_middleware: Vec::new(),
			pre_middleware: Vec::new(),
			error_handler: None,
		}
	}

	pub fn middleware(mut self, middleware: Middleware<O, E>) -> Self {
		match middleware {
			Middleware::Pre(handler) => self.pre_middleware.push(handler),
			Middleware::Post(handler) => self.post_middleware.push(handler),
		}

		self
	}

	pub fn data<T: Clone + Send + Sync + 'static>(self, data: T) -> Self {
		self.middleware(Middleware::pre(move |mut req| {
			req.extensions_mut().insert(data.clone());
			async move { Ok(req) }
		}))
	}

	pub fn error_handler<F: std::future::Future<Output = hyper::Response<O>> + Send + 'static>(
		mut self,
		handler: impl Fn(hyper::Request<()>, E) -> F + Send + Sync + 'static,
	) -> Self {
		self.error_handler = Some(ErrorHandler(Box::new(move |(req, err)| Box::pin(handler(req, err)))));
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
				handler: RouteHandler(Box::new(move |req| Box::pin(handler(req)))),
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

	fn build_scoped(
		mut self,
		parent_path: &str,
		target: &mut Router<I, O, E>,
		pre_middlewares: &[usize],
		post_middlewares: &[usize],
		error_handler: Option<usize>,
	) {
		let error_handler = if let Some(error_handler) = self.error_handler.take() {
			target.error_handlers.push(error_handler);
			Some(target.error_handlers.len() - 1)
		} else {
			error_handler
		};

		let pre_middleware_idxs = pre_middlewares
			.iter()
			.copied()
			.chain(self.pre_middleware.into_iter().map(|handler| {
				target.pre_middlewares.push(handler);
				target.pre_middlewares.len() - 1
			}))
			.collect::<Vec<_>>();

		let post_middleware_idxs = post_middlewares
			.iter()
			.copied()
			.chain(self.post_middleware.into_iter().map(|handler| {
				target.post_middlewares.push(handler);
				target.post_middlewares.len() - 1
			}))
			.collect::<Vec<_>>();

		for (path, item) in self.tree.drain(..) {
			match item {
				RouterItem::Route(route) => {
					target.routes.push(route.handler);

					let info = RouteInfo {
						route: target.routes.len() - 1,
						pre_middleware: pre_middleware_idxs.clone(),
						post_middleware: post_middleware_idxs.clone(),
						error_handler,
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
						if parent_path.is_empty() { "" } else { "/" },
						path
					);

					tracing::debug!(parent_path, path, full_path, "adding route");

					let _ = target.tree.insert(&full_path, info);
				}
				RouterItem::Router(router) => {
					let parent_path = parent_path.trim_matches('/');
					let path = path.trim_matches('/');
					router.build_scoped(
						&format!("{parent_path}{}{path}", if parent_path.is_empty() { "" } else { "/" }),
						target,
						&pre_middleware_idxs,
						&post_middleware_idxs,
						error_handler,
					);
				}
			}
		}
	}

	pub fn build(self) -> Router<I, O, E> {
		let mut router = Router {
			routes: Vec::new(),
			pre_middlewares: Vec::new(),
			post_middlewares: Vec::new(),
			error_handlers: Vec::new(),
			tree: path_tree::PathTree::new(),
		};

		self.build_scoped("", &mut router, &[], &[], None);

		router
	}
}
