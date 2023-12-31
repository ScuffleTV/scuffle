use std::sync::{Arc, Mutex};

use common::http::router::ext::RequestExt as _;
use common::http::router::middleware::Middleware;
use common::http::RouteError;
use hyper::header::IntoHeaderName;
use hyper::Request;

use crate::api::error::ApiError;
use crate::api::Body;
use crate::global::ApiGlobal;

#[derive(Clone)]
pub struct ResponseHeadersMiddleware(pub Arc<Mutex<hyper::HeaderMap>>);

impl Default for ResponseHeadersMiddleware {
	fn default() -> Self {
		Self(Arc::new(Mutex::new(hyper::HeaderMap::new())))
	}
}

pub fn pre_flight_middleware<G: ApiGlobal>(_: &Arc<G>) -> Middleware<Body, RouteError<ApiError>> {
	Middleware::pre(|mut req| async move {
		req.extensions_mut().insert(ResponseHeadersMiddleware::default());

		Ok(req)
	})
}

pub fn post_flight_middleware<G: ApiGlobal>(_: &Arc<G>) -> Middleware<Body, RouteError<ApiError>> {
	Middleware::post_with_req(|mut resp, req| async move {
		let headers = req.data::<ResponseHeadersMiddleware>();

		if let Some(headers) = headers {
			let headers = headers.0.lock().expect("failed to lock headers");
			headers.iter().for_each(|(k, v)| {
				resp.headers_mut().insert(k, v.clone());
			});
		}

		Ok(resp)
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
