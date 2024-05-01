mod batch_loader;
mod types;
mod utils;

use std::collections::{HashMap, HashSet};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Duration;

use futures::FutureExt;

use self::batch_loader::BatchLoader;
pub use self::types::LoaderOutput;
use self::types::{BatchState, DataLoaderInner};
use self::utils::new_auto_loader;
use crate::runtime;

pub trait Loader: Send + Sync + 'static {
	type Key: Eq + std::hash::Hash + Clone + Send + Sync;
	type Value: Clone + Send + Sync;
	type Error: Clone + Send + Sync;

	fn load(&self, key: Vec<Self::Key>) -> impl std::future::Future<Output = LoaderOutput<Self>> + Send;
}

pub struct DataLoader<L: Loader> {
	batch_id: AtomicU64,
	loader: Arc<L>,
	max_batch_size: usize,
	inner: DataLoaderInner<L>,
	_auto_loader_abort: CancelOnDrop,
	name: String,
}

struct CancelOnDrop(tokio::task::AbortHandle);

impl Drop for CancelOnDrop {
	fn drop(&mut self) {
		self.0.abort();
	}
}

impl<L: Loader + Default> Default for DataLoader<L> {
	fn default() -> Self {
		Self::new(std::any::type_name::<L>(), L::default())
	}
}

impl<L: Loader> DataLoader<L> {
	pub fn new(name: impl ToString, loader: L) -> Self {
		Self::with_concurrency_limit(name, loader, 10)
	}

	pub fn with_concurrency_limit(name: impl ToString, loader: L, concurrency_limit: usize) -> Self {
		let duration = Duration::from_millis(5);

		let inner = DataLoaderInner::new(concurrency_limit, duration);

		Self {
			batch_id: AtomicU64::new(0),
			loader: Arc::new(loader),
			max_batch_size: 1000,
			_auto_loader_abort: new_auto_loader(inner.clone()),
			inner,
			name: name.to_string(),
		}
	}

	pub fn set_max_batch_size(mut self, max_batch_size: usize) -> Self {
		self.max_batch_size = max_batch_size;
		self
	}

	pub fn set_duration(self, duration: Duration) -> Self {
		self.inner
			.duration
			.store(duration.as_nanos() as u64, std::sync::atomic::Ordering::Relaxed);
		self
	}

	async fn extend_loader(&self, keys: impl Iterator<Item = L::Key>) -> Vec<(Vec<L::Key>, BatchState<L>)> {
		let mut batches = Vec::new();
		let mut current_batch = None;

		let mut active_batch = self.inner.active_batch.write().await;

		for key in keys {
			if active_batch
				.as_ref()
				.map(|b| b.keys.len() >= self.max_batch_size)
				.unwrap_or(true)
			{
				let batch = BatchLoader {
					id: self.batch_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
					loader: self.loader.clone(),
					keys: Default::default(),
					start: tokio::time::Instant::now(),
					state: Default::default(),
				};

				if let Some(current_batch) = current_batch.replace((Vec::new(), batch.state.clone())) {
					batches.push(current_batch);
				}

				if let Some(batch) = active_batch.replace(batch) {
					runtime::spawn(batch.load(self.inner.semaphore.clone()));
				}

				self.inner.notify.notify_waiters();
			} else if current_batch.is_none() {
				current_batch = Some((Vec::new(), active_batch.as_ref().unwrap().state.clone()));
			}

			let (Some(active_batch), Some((current_batch, _))) = (active_batch.as_mut(), current_batch.as_mut()) else {
				unreachable!();
			};

			active_batch.keys.insert(key.clone());
			current_batch.push(key);
		}

		if let Some(current_batch) = current_batch.take() {
			batches.push(current_batch);
		}

		if let Some(batch) = active_batch.as_mut() {
			if batch.keys.len() > self.max_batch_size {
				let batch = active_batch.take().unwrap();
				runtime::spawn(batch.load(self.inner.semaphore.clone()));
			}
		}

		batches
	}

	async fn internal_load(&self, keys: impl IntoIterator<Item = L::Key>) -> LoaderOutput<L> {
		let key_set = keys.into_iter().collect::<HashSet<_>>().into_iter().collect::<Vec<_>>();

		if key_set.is_empty() {
			return Ok(Default::default());
		}

		let batches = self.extend_loader(key_set.iter().cloned()).await;

		let batches = futures::future::join_all(
			batches
				.iter()
				.map(|(keys, batch)| batch.wait().map(move |result| result.map(|result| (keys, result)))),
		)
		.await;

		batches
			.into_iter()
			.flatten()
			.try_fold(HashMap::new(), |mut acc, (keys, batch)| match batch {
				Ok(batch) => {
					acc.extend(
						keys.into_iter()
							.cloned()
							.filter_map(|key| batch.get(&key).cloned().map(|value| (key, value))),
					);

					Ok(acc)
				}
				Err(err) => Err(err.clone()),
			})
	}

	#[tracing::instrument(skip(self, keys), fields(name = self.name.as_str()))]
	pub async fn load_many(&self, keys: impl IntoIterator<Item = L::Key>) -> LoaderOutput<L> {
		self.internal_load(keys).await
	}

	#[tracing::instrument(skip(self, key), fields(name = self.name.as_str()))]
	pub async fn load(&self, key: L::Key) -> Result<Option<L::Value>, L::Error> {
		Ok(self.internal_load(std::iter::once(key.clone())).await?.remove(&key))
	}
}

#[cfg(test)]
mod tests {
	use std::collections::hash_map::RandomState;
	use std::collections::HashMap;

	use crate::dataloader::LoaderOutput;

	type DynBoxLoader<S = RandomState> = Box<dyn Fn(Vec<u64>) -> HashMap<u64, u64, S> + Sync + Send>;

	struct LoaderTest {
		results: DynBoxLoader,
	}

	impl crate::dataloader::Loader for LoaderTest {
		type Error = ();
		type Key = u64;
		type Value = u64;

		async fn load(&self, keys: Vec<Self::Key>) -> LoaderOutput<Self> {
			tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
			Ok((self.results)(keys))
		}
	}

	#[tokio::test]
	async fn test_data_loader() {
		let run_count = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));

		let loader = LoaderTest {
			results: Box::new(move |keys| {
				let mut results = HashMap::new();

				assert!(run_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0);

				assert!(keys.len() <= 1000);

				for key in keys {
					assert!(!results.contains_key(&key));

					results.insert(key, key * 2);
				}

				results
			}),
		};

		let dataloader = crate::dataloader::DataLoader::new("test", loader);

		let futures = (0..250)
			.map(|i| dataloader.load(i as u64))
			.chain((0..250).map(|i| dataloader.load(i as u64)));

		let results = futures::future::join_all(futures).await;

		let expected = (0..250)
			.map(|i| Ok(Some(i * 2)))
			.chain((0..250).map(|i| Ok(Some(i * 2))))
			.collect::<Vec<_>>();

		assert_eq!(results, expected);
	}

	#[tokio::test]
	async fn test_data_loader_larger() {
		let loader = LoaderTest {
			results: Box::new(move |keys| {
				let mut results = HashMap::new();

				assert!(keys.len() <= 1000);

				for key in keys {
					assert!(!results.contains_key(&key));

					results.insert(key, key * 2);
				}

				results
			}),
		};

		let dataloader = crate::dataloader::DataLoader::new("test", loader);

		const LIMIT: usize = 10_000;

		let results = futures::future::join_all((0..LIMIT).map(|i| dataloader.load(i as u64))).await;

		let expected = (0..LIMIT).map(|i| Ok(Some(i as u64 * 2))).collect::<Vec<_>>();

		assert_eq!(results, expected);
	}

	#[tokio::test]
	async fn test_data_loader_change_batch_size() {
		let loader = LoaderTest {
			results: Box::new(move |keys| {
				let mut results = HashMap::new();

				assert!(keys.len() <= 3000);

				for key in keys {
					assert!(!results.contains_key(&key));

					results.insert(key, key * 2);
				}

				results
			}),
		};

		let dataloader = crate::dataloader::DataLoader::new("test", loader).set_max_batch_size(3000);

		let futures = (0..5000).map(|i| dataloader.load(i as u64));

		let results = futures::future::join_all(futures).await;

		let expected = (0..5000).map(|i| Ok(Some(i * 2))).collect::<Vec<_>>();

		assert_eq!(results, expected);
	}

	#[tokio::test]
	async fn test_data_loader_change_duration() {
		let loader = LoaderTest {
			results: Box::new(move |keys| {
				let mut results = HashMap::new();

				assert!(keys.len() <= 1000);

				for key in keys {
					assert!(!results.contains_key(&key));

					results.insert(key, key * 2);
				}

				results
			}),
		};

		let dataloader =
			crate::dataloader::DataLoader::new("test", loader).set_duration(tokio::time::Duration::from_millis(100));

		let futures = (0..250)
			.map(|i| dataloader.load(i as u64))
			.chain((0..250).map(|i| dataloader.load(i as u64)));

		let results = futures::future::join_all(futures).await;

		let expected = (0..250)
			.map(|i| Ok(Some(i * 2)))
			.chain((0..250).map(|i| Ok(Some(i * 2))))
			.collect::<Vec<_>>();

		assert_eq!(results, expected);
	}

	#[tokio::test]
	async fn test_data_loader_value_deduplication() {
		let run_count = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));

		let loader = LoaderTest {
			results: Box::new({
				let run_count = run_count.clone();
				move |keys| {
					run_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
					keys.iter().map(|&key| (key, key * 2)).collect()
				}
			}),
		};

		let dataloader = crate::dataloader::DataLoader::new("test", loader);

		let futures = vec![dataloader.load(5), dataloader.load(5), dataloader.load(5)];

		let results: Vec<_> = futures::future::join_all(futures).await;

		assert_eq!(results, vec![Ok(Some(10)), Ok(Some(10)), Ok(Some(10))]);
		assert_eq!(run_count.load(std::sync::atomic::Ordering::SeqCst), 1); // Ensure the loader was only called once
	}
}
