#[derive(Debug, Clone)]
pub struct RouteParams(pub Box<[(String, String)]>);

#[derive(Debug)]
pub(crate) struct RouteInfo {
	pub route: usize,
	pub middleware: Vec<usize>,
}
