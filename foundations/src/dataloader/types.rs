use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, OnceLock};

use tokio::sync::Notify;
use tokio::time::Instant;

use super::batch_loader::BatchLoader;
use super::Loader;

#[allow(type_alias_bounds)]
pub type LoaderOutput<L: Loader> = Result<HashMap<L::Key, L::Value>, L::Error>;

pub struct DataLoaderInternal<L: Loader> {
	pub active_batch: tokio::sync::RwLock<Option<BatchLoader<L>>>,
	pub notify: tokio::sync::Notify,
	pub semaphore: Arc<tokio::sync::Semaphore>,
	pub duration: AtomicU64,
}

pub(super) struct DataLoaderInner<L: Loader>(Arc<DataLoaderInternal<L>>);

impl<L: Loader> Clone for DataLoaderInner<L> {
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}

impl<L: Loader> DataLoaderInner<L> {
	pub fn new(concurrency: usize, duration: std::time::Duration) -> Self {
		Self(Arc::new(DataLoaderInternal {
			active_batch: Default::default(),
			notify: Default::default(),
			semaphore: Arc::new(tokio::sync::Semaphore::new(concurrency)),
			duration: AtomicU64::new(duration.as_nanos() as u64),
		}))
	}

	pub async fn load_active_batch(&self) -> Option<(u64, Instant)> {
		self.active_batch.read().await.as_ref().map(|b| (b.id, b.start))
	}

	pub fn duration(&self) -> std::time::Duration {
		std::time::Duration::from_nanos(self.0.duration.load(std::sync::atomic::Ordering::Relaxed))
	}
}

impl<L: Loader> std::ops::Deref for DataLoaderInner<L> {
	type Target = DataLoaderInternal<L>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

struct InternalBatchState<L: Loader> {
	notify: Notify,
	result: OnceLock<Option<LoaderOutput<L>>>,
}

pub(super) struct BatchState<L: Loader>(Arc<InternalBatchState<L>>);

impl<L: Loader> Default for BatchState<L> {
	fn default() -> Self {
		Self(Arc::new(InternalBatchState {
			notify: Notify::new(),
			result: OnceLock::new(),
		}))
	}
}

impl<L: Loader> Clone for BatchState<L> {
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}

impl<L: Loader> BatchState<L> {
	pub fn notify(&self, result: Option<LoaderOutput<L>>) -> bool {
		if self.0.result.set(result).is_ok() {
			self.0.notify.notify_waiters();
			true
		} else {
			false
		}
	}

	pub async fn wait(&self) -> Option<&LoaderOutput<L>> {
		let notify = self.0.notify.notified();
		if let Some(result) = self.0.result.get() {
			return result.as_ref();
		}

		notify.await;

		match self.0.result.get() {
			Some(result) => result.as_ref(),
			None => None,
		}
	}
}
