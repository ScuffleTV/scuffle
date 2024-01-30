use super::builder::RouterBuilder;

pub trait ExtendRouter<I, O, E> {
	fn extend(&mut self, router: RouterBuilder<I, O, E>) -> RouterBuilder<I, O, E>;
}

impl<I, O, E, F: Fn(RouterBuilder<I, O, E>) -> RouterBuilder<I, O, E>> ExtendRouter<I, O, E> for F {
	fn extend(&mut self, router: RouterBuilder<I, O, E>) -> RouterBuilder<I, O, E> {
		self(router)
	}
}

pub fn extend_fn<I, O, E, F: Fn(RouterBuilder<I, O, E>) -> RouterBuilder<I, O, E>>(f: F) -> impl ExtendRouter<I, O, E> {
	f
}
