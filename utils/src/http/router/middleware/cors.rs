use http::HeaderValue;
use hyper::http::header;

use super::{Middleware, NextFn};
use crate::http::router::builder::RouterBuilder;

#[derive(Clone)]
pub struct CorsOptions<B> {
	/// Allow headers
	pub allow_headers: Vec<String>,
	/// Allow methods
	pub allow_methods: Vec<String>,
	/// Allow origin
	pub allow_origin: Vec<String>,
	/// Expose headers
	pub expose_headers: Vec<String>,
	/// Max age seconds
	pub max_age_seconds: Option<u64>,
	/// Timing allow origin
	pub timing_allow_origin: Vec<String>,

	// Default response body
	pub default_response: fn() -> B,
}

impl<B: Default> Default for CorsOptions<B> {
	fn default() -> Self {
		Self {
			allow_headers: Vec::new(),
			allow_methods: Vec::new(),
			allow_origin: Vec::new(),
			expose_headers: Vec::new(),
			max_age_seconds: None,
			timing_allow_origin: Vec::new(),
			default_response: || B::default(),
		}
	}
}

impl<B> CorsOptions<B> {
	pub fn wildcard() -> Self
	where
		B: Default,
	{
		Self::wildcard_with_default_response(|| B::default())
	}

	pub fn wildcard_with_default_response(default_response: fn() -> B) -> Self {
		Self {
			allow_headers: vec!["*".to_string()],
			allow_methods: vec!["*".to_string()],
			allow_origin: vec!["*".to_string()],
			expose_headers: vec!["*".to_string()],
			max_age_seconds: Some(3600),
			timing_allow_origin: vec!["*".to_string()],
			default_response,
		}
	}
}

pub struct CorsMiddleware<B> {
	allow_origins: fnv::FnvHashSet<String>,
	allow_methods: HeaderValue,
	allow_headers: HeaderValue,
	expose_headers: HeaderValue,
	max_age: Option<HeaderValue>,
	timing_allow_origins: fnv::FnvHashSet<String>,
	default_response: fn() -> B,
}

impl<B> CorsMiddleware<B> {
	pub fn new(options: &CorsOptions<B>) -> Self {
		let allow_origins = fnv::FnvHashSet::from_iter(options.allow_origin.iter().map(|s| s.to_lowercase()));
		let allow_methods = options.allow_methods.join(", ").parse::<HeaderValue>().unwrap();
		let allow_headers = options.allow_headers.join(", ").parse::<HeaderValue>().unwrap();
		let expose_headers = options.expose_headers.join(", ").parse::<HeaderValue>().unwrap();
		let max_age = options.max_age_seconds.map(|s| s.to_string().parse::<HeaderValue>().unwrap());
		let timing_allow_origins = fnv::FnvHashSet::from_iter(options.timing_allow_origin.iter().map(|s| s.to_lowercase()));

		Self {
			allow_origins,
			allow_methods,
			allow_headers,
			expose_headers,
			max_age,
			timing_allow_origins,
			default_response: options.default_response,
		}
	}
}

#[async_trait::async_trait]
impl<I: Send + 'static, O: Default + Send + 'static, E: Send + 'static> Middleware<I, O, E> for CorsMiddleware<O> {
	async fn handle(&self, req: hyper::Request<I>, next: NextFn<I, O, E>) -> Result<hyper::Response<O>, E> {
		let origin = match req.headers().get(header::ORIGIN) {
			Some(origin) => Some(origin.clone()),
			None => None,
		};

		let mut resp = next(req).await?;

		if self.allow_origins.is_empty() {
			return Ok(resp);
		}

		let origin = match origin {
			Some(origin) => origin,
			None => return Ok(resp),
		};

		let origin_str = origin.to_str().unwrap().to_lowercase();

		if !self.allow_origins.contains("*") && !self.allow_origins.contains(&origin_str) {
			return Ok(resp);
		}

		resp.headers_mut().insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin.clone());

		if self.timing_allow_origins.contains("*") || self.timing_allow_origins.contains(&origin_str) {
			resp.headers_mut().insert("Timing-Allow-Origin", origin.clone());
		}

		if !self.allow_methods.is_empty() {
			resp.headers_mut()
				.insert(header::ACCESS_CONTROL_ALLOW_METHODS, self.allow_methods.clone());
		}

		if !self.allow_headers.is_empty() {
			resp.headers_mut()
				.insert(header::ACCESS_CONTROL_ALLOW_HEADERS, self.allow_headers.clone());
		}

		if !self.expose_headers.is_empty() {
			resp.headers_mut()
				.insert(header::ACCESS_CONTROL_EXPOSE_HEADERS, self.expose_headers.clone());
		}

		if let Some(max_age) = self.max_age.clone() {
			resp.headers_mut().insert(header::ACCESS_CONTROL_MAX_AGE, max_age);
		}

		resp.headers_mut().insert(header::VARY, "Origin".parse().unwrap());

		Ok(resp)
	}

	fn extend(&self, builder: RouterBuilder<I, O, E>) -> RouterBuilder<I, O, E> {
		let func = self.default_response;
		builder.options("/*", move |_| async move {
			Ok(hyper::Response::builder()
				.status(hyper::StatusCode::NO_CONTENT)
				.body((func)())
				.unwrap())
		})
	}
}
