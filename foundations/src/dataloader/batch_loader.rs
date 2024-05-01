use std::collections::HashSet;
use std::sync::Arc;

use super::types::BatchState;
use super::Loader;

pub(super) struct BatchLoader<L: Loader> {
	pub id: u64,
	pub loader: Arc<L>,
	pub keys: HashSet<L::Key>,
	pub start: tokio::time::Instant,
	pub state: BatchState<L>,
}

impl<L: Loader> BatchLoader<L> {
	pub async fn load(self, sephamore: Arc<tokio::sync::Semaphore>) {
		let _ticket = sephamore.acquire().await.unwrap();
		let keys = self.keys.into_iter().collect::<Vec<_>>();
		let result = self.loader.load(keys).await;
		self.state.notify(Some(result));
	}
}
