use super::types::RouteParams;

pub trait RequestExt {
	fn param(&self, key: &str) -> Option<&str> {
		self.params().0.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
	}

	fn params(&self) -> &RouteParams;

	fn provide<T: Send + Sync + Clone + 'static>(&mut self, data: T);

	fn data<T: Send + Sync + 'static>(&self) -> Option<&T>;
}

impl<I> RequestExt for hyper::Request<I> {
	fn params(&self) -> &RouteParams {
		self.extensions().get::<RouteParams>().unwrap()
	}

	fn provide<T: Send + Sync + Clone + 'static>(&mut self, data: T) {
		self.extensions_mut().insert(data);
	}

	fn data<T: Send + Sync + 'static>(&self) -> Option<&T> {
		self.extensions().get::<T>()
	}
}
