use std::fmt::{Debug, Formatter};

use error::RouterError;
use route::RouteHandler;

use self::builder::RouterBuilder;
use self::middleware::{PostMiddlewareHandler, PreMiddlewareHandler};
use self::types::{ErrorHandler, RouteInfo, RouteParams};

pub mod builder;
pub mod compat;
pub mod error;
pub mod ext;
pub mod extend;
pub mod middleware;
pub mod route;
pub mod types;

pub struct Router<I, O, E> {
	routes: Vec<RouteHandler<I, O, E>>,
	pre_middlewares: Vec<PreMiddlewareHandler<E>>,
	post_middlewares: Vec<PostMiddlewareHandler<O, E>>,
	error_handlers: Vec<ErrorHandler<O, E>>,
	tree: path_tree::PathTree<RouteInfo>,
}

impl<I: 'static, O: 'static, E: 'static> Router<I, O, E> {
	pub fn builder() -> RouterBuilder<I, O, E> {
		RouterBuilder::new()
	}

	pub async fn handle(&self, mut req: hyper::Request<I>) -> Result<hyper::Response<O>, RouterError<E>> {
		let path = format!("/{}{}", req.method().as_str(), req.uri().path());
		let (info, path) = self.tree.find(&path).ok_or(RouterError::NotFound)?;

		req.extensions_mut().insert(RouteParams(
			path.params_iter().map(|(k, v)| (k.to_owned(), v.to_owned())).collect(),
		));

		let error_handler = info.error_handler.map(|idx| self.error_handlers[idx].0.as_ref());

		for idx in info.pre_middleware.iter().copied() {
			let (parts, body) = req.into_parts();
			req = match self.pre_middlewares[idx].0(hyper::Request::from_parts(parts.clone(), ())).await {
				Ok(req) => {
					let (parts, _) = req.into_parts();
					hyper::Request::from_parts(parts, body)
				}
				Err(err) => {
					if let Some(error_handler) = error_handler {
						return Ok(error_handler((hyper::Request::from_parts(parts, ()), err)).await);
					} else {
						return Err(RouterError::Unhandled(err));
					}
				}
			};
		}

		let (parts, body) = req.into_parts();

		let req = hyper::Request::from_parts(parts.clone(), ());

		let mut res = match self.routes[info.route].0(hyper::Request::from_parts(parts, body)).await {
			Ok(res) => res,
			Err(err) => {
				if let Some(error_handler) = error_handler {
					error_handler((req.clone(), err)).await
				} else {
					return Err(RouterError::Unhandled(err));
				}
			}
		};

		for idx in info.post_middleware.iter().copied() {
			let (parts, body) = res.into_parts();
			res = match self.post_middlewares[idx].0((hyper::Response::from_parts(parts.clone(), body), req.clone())).await {
				Ok(res) => res,
				Err(err) => {
					if let Some(error_handler) = error_handler {
						return Ok(error_handler((req, err)).await);
					} else {
						return Err(RouterError::Unhandled(err));
					}
				}
			};
		}

		Ok(res)
	}
}

impl<I, O, E> Debug for Router<I, O, E> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Router")
			.field("routes", &self.routes)
			.field("pre_middlewares", &self.pre_middlewares)
			.field("post_middlewares", &self.post_middlewares)
			.field("error_handlers", &self.error_handlers)
			.field("tree", &self.tree)
			.finish()
	}
}
