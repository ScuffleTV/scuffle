use std::sync::{Arc, Mutex};

use common::http::RouteError;
use hyper::{header::IntoHeaderName, Body, Request};
use routerify::{prelude::RequestExt as _, Middleware};

use crate::{api::error::ApiError, global::ApiGlobal};

#[derive(Clone)]
pub struct ResponseHeadersMiddleware(pub Arc<Mutex<hyper::HeaderMap>>);

impl Default for ResponseHeadersMiddleware {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(hyper::HeaderMap::new())))
    }
}

pub fn pre_flight_middleware<G: ApiGlobal>(_: &Arc<G>) -> Middleware<Body, RouteError<ApiError>> {
    Middleware::pre(|req| async move {
        req.set_context(ResponseHeadersMiddleware::default());

        Ok(req)
    })
}

pub fn post_flight_middleware<G: ApiGlobal>(_: &Arc<G>) -> Middleware<Body, RouteError<ApiError>> {
    Middleware::post_with_info(|mut resp, info| async move {
        let headers: Option<ResponseHeadersMiddleware> = info.context();

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

impl RequestExt for Request<Body> {
    fn set_response_header<K, V>(&self, key: K, value: V)
    where
        K: IntoHeaderName,
        V: Into<hyper::header::HeaderValue>,
    {
        let headers: ResponseHeadersMiddleware = self.context().unwrap();

        let mut headers = headers.0.lock().expect("failed to lock headers");
        key.insert(&mut headers, value.into());
    }
}
