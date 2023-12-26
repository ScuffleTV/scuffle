mod batch_loader;
mod cache;
mod types;
mod utils;

use std::collections::hash_map::RandomState;
use std::collections::{HashMap, HashSet};
use std::hash::BuildHasher;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Duration;

use batch_loader::BatchLoader;
pub use cache::{Cache, HashMapCache, NoCache, SharedCache};
use tokio::sync::{mpsc, Mutex, RwLock};
pub use types::LoaderOutput;
use types::{BatchState, DataLoaderInnerHolder};

use self::types::DataLoaderInner;
use self::utils::new_auto_loader;

pub trait Loader<S = RandomState>: Send + Sync + 'static {
	type Key: Eq + std::hash::Hash + Clone + Sync + Send;
	type Value: Clone + Sync + Send;
	type Error: Clone + Sync + Send;

	fn load(&self, key: &[Self::Key]) -> impl std::future::Future<Output = LoaderOutput<Self, S>> + Send;
}

pub struct DataLoader<L: Loader<S>, S = RandomState> {
	batch_id: AtomicU64,
	loader: Arc<L>,
	max_batch_size: usize,
	inner: DataLoaderInnerHolder<L, S>,
	new_batch: mpsc::Sender<()>,
	_auto_loader_abort: tokio::task::AbortHandle,
}

impl<L: Loader<S> + Default, S: Send + Sync + Default + BuildHasher + 'static> Default for DataLoader<L, S> {
	fn default() -> Self {
		Self::new(L::default())
	}
}

impl<L: Loader<S>, S: Send + Sync + Default + BuildHasher + 'static> DataLoader<L, S> {
	pub fn new(loader: L) -> Self {
		Self::with_concurrency_limit(loader, 10)
	}

	pub fn with_concurrency_limit(loader: L, concurrency_limit: usize) -> Self {
		let duration = Duration::from_millis(5);

		let (auto_loader_tx, auto_loader_rx) = mpsc::channel(1);

		let inner = Arc::new(Mutex::new(DataLoaderInner {
			active_batch: None,
			semaphore: Arc::new(tokio::sync::Semaphore::new(concurrency_limit)),
		}));

		Self {
			batch_id: AtomicU64::new(0),
			loader: Arc::new(loader),
			max_batch_size: 1000,
			new_batch: auto_loader_tx,
			_auto_loader_abort: new_auto_loader(auto_loader_rx, duration, inner.clone()),
			inner,
		}
	}

	pub fn set_max_batch_size(mut self, max_batch_size: usize) -> Self {
		self.max_batch_size = max_batch_size;
		self
	}

	pub fn set_duration(mut self, duration: Duration) -> Self {
		let (auto_loader_tx, auto_loader_rx) = mpsc::channel(1);

		self.new_batch = auto_loader_tx;
		self._auto_loader_abort = new_auto_loader(auto_loader_rx, duration, self.inner.clone());

		self
	}

	async fn extend_loader(&self, keys: impl Iterator<Item = L::Key>) -> Vec<BatchState<L, S>> {
		let mut inner = self.inner.lock().await;

		let mut batches = HashMap::<_, _, S>::default();

		for key in keys {
			if inner
				.active_batch
				.as_ref()
				.map(|b| b.keys.len() >= self.max_batch_size)
				.unwrap_or(true)
			{
				let batch_id = self.batch_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

				let batch = BatchLoader {
					id: batch_id,
					loader: self.loader.clone(),
					keys: HashSet::default(),
					start: tokio::time::Instant::now(),
					result: Arc::new(RwLock::new(None)),
					token: tokio_util::sync::CancellationToken::new(),
				};

				if let Some(batch) = inner.active_batch.take() {
					tokio::spawn(batch.load(inner.semaphore.clone()));
				}

				inner.active_batch = Some(batch);
				self.new_batch.try_send(()).ok();
			}

			let batch = inner.active_batch.as_mut().unwrap();
			batch.keys.insert(key);

			if let std::collections::hash_map::Entry::Vacant(e) = batches.entry(batch.id) {
				e.insert((batch.result.clone(), batch.token.clone().cancelled_owned()));
			}

			if batch.keys.len() >= self.max_batch_size {
				let batch = inner.active_batch.take().unwrap();
				tokio::spawn(batch.load(inner.semaphore.clone()));
			}
		}

		batches.into_values().collect()
	}

	#[inline(always)]
	pub async fn load_many(&self, keys: impl Iterator<Item = L::Key>) -> LoaderOutput<L, S> {
		self.load_many_with_cache(NoCache, keys).await
	}

	pub async fn load_many_with_cache<C: Cache<L, S>>(
		&self,
		mut cache: C,
		keys: impl Iterator<Item = L::Key>,
	) -> LoaderOutput<L, S> {
		let mut results = HashMap::default();

		let mut key_set = HashSet::<_, S>::default();
		for key in keys {
			if let Some(value) = cache.get(&key) {
				results.insert(key, value);
			} else {
				key_set.insert(key);
			}
		}

		if key_set.is_empty() {
			return Ok(results);
		}

		let batches = self.extend_loader(key_set.iter().cloned()).await;

		for (result, rx) in batches {
			// If the receiver is closed, the batch has already been loaded
			rx.await;

			// We try do do an unwrap first, because it's faster and more memory efficient
			// If that fails it means we are not the only one waiting for the result
			match Arc::try_unwrap(result) {
				Ok(result) => {
					let result = result.into_inner().unwrap()?;
					result.into_iter().for_each(|(k, v)| {
						if key_set.contains(&k) {
							cache.insert(&k, &v);
							results.insert(k, v);
						}
					});
				}
				Err(result) => {
					let result = result.read().await;
					let result = result.as_ref().unwrap().as_ref().map_err(|e| e.clone())?;

					result.iter().for_each(|(k, v)| {
						if key_set.contains(k) {
							cache.insert(k, v);
							results.insert(k.clone(), v.clone());
						}
					});
				}
			}
		}

		Ok(results)
	}

	#[inline(always)]
	pub async fn load_with_cache<C: Cache<L, S>>(&self, cache: C, key: L::Key) -> Result<Option<L::Value>, L::Error> {
		Ok(self
			.load_many_with_cache(cache, std::iter::once(key.clone()))
			.await?
			.remove(&key))
	}

	#[inline(always)]
	pub async fn load(&self, key: L::Key) -> Result<Option<L::Value>, L::Error> {
		self.load_with_cache(NoCache, key).await
	}
}
