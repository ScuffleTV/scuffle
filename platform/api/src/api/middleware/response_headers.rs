use std::sync::{Arc, Mutex};

use common::http::router::ext::RequestExt as _;
use common::http::router::extend::{extend_fn, ExtendRouter};
use common::http::router::middleware::Middleware;
use common::http::RouteError;
use hyper::body::Incoming;
use hyper::header::IntoHeaderName;
use hyper::Request;

use crate::api::error::ApiError;
use crate::api::Body;
use crate::global::ApiGlobal;

#[derive(Clone, Default)]
struct ResponseHeadersMiddleware(Arc<Mutex<hyper::HeaderMap>>);

pub fn response_headers<G: ApiGlobal>(_: &Arc<G>) -> impl ExtendRouter<Incoming, Body, RouteError<ApiError>> {
	extend_fn(|router| {
		router
			.middleware(Middleware::pre(|mut req| async move {
				req.extensions_mut().insert(ResponseHeadersMiddleware::default());

				Ok(req)
			}))
			.middleware(Middleware::post_with_req(|mut resp, req| async move {
				let headers = req.data::<ResponseHeadersMiddleware>();

				if let Some(headers) = headers {
					let headers = headers.0.lock().expect("failed to lock headers");
					headers.iter().for_each(|(k, v)| {
						resp.headers_mut().insert(k, v.clone());
					});
				}

				Ok(resp)
			}))
	})
}

pub trait RequestExt {
	fn set_response_header<K, V>(&self, key: K, value: V)
	where
		K: IntoHeaderName,
		V: Into<hyper::header::HeaderValue>;
}

impl<B> RequestExt for Request<B> {
	fn set_response_header<K, V>(&self, key: K, value: V)
	where
		K: IntoHeaderName,
		V: Into<hyper::header::HeaderValue>,
	{
		let headers: &ResponseHeadersMiddleware = self.data().unwrap();

		let mut headers = headers.0.lock().expect("failed to lock headers");
		key.insert(&mut headers, value.into());
	}
}
