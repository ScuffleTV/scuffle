#[derive(thiserror::Error, Debug)]
pub enum RouterError<E> {
	#[error("Unhandled error: {0:?}")]
	Unhandled(E),
	#[error("Route not found")]
	NotFound,
}
