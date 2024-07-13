use std::collections::HashMap;
use std::hash::{BuildHasher, RandomState};
use std::marker::PhantomData;

use super::{BatchOperation, Batcher, BatcherConfig, BatcherDataloader, BatcherError};

#[allow(type_alias_bounds)]
pub type LoaderOutput<L: Loader<S>, S: BuildHasher = RandomState> = Result<HashMap<L::Key, L::Value, S>, L::Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct UnitError;

impl std::error::Error for UnitError {}

impl std::fmt::Display for UnitError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "unknown")
	}
}

impl From<()> for UnitError {
	fn from(_: ()) -> Self {
		Self
	}
}

pub trait Loader<S: BuildHasher + Default = RandomState> {
	type Key: Clone + Eq + std::hash::Hash + Send + Sync;
	type Value: Clone + Send + Sync;
	type Error: Clone + std::error::Error + Send + Sync;

	fn config(&self) -> BatcherConfig {
		BatcherConfig {
            name: std::any::type_name::<Self>().to_string(),
			concurrency: 10,
			max_batch_size: 1000,
			sleep_duration: std::time::Duration::from_millis(5),
		}
	}

	fn load(&self, keys: Vec<Self::Key>) -> impl std::future::Future<Output = LoaderOutput<Self, S>> + Send;
}

pub struct DataLoader<L: Loader<S>, S: BuildHasher + Default + Send + Sync = RandomState> {
	batcher: Batcher<Wrapper<L, S>>,
}

impl<L: Loader<S> + 'static + Send + Sync, S: BuildHasher + Default + Send + Sync + 'static> DataLoader<L, S> {
	pub fn new(loader: L) -> Self {
		Self {
			batcher: Batcher::new(Wrapper(loader, PhantomData)),
		}
	}

	pub async fn load(&self, key: L::Key) -> Result<Option<L::Value>, BatcherError<L::Error>> {
		self.load_many(std::iter::once(key.clone()))
			.await
			.map(|mut map| map.remove(&key))
	}

	pub async fn load_many(
		&self,
		keys: impl IntoIterator<Item = L::Key>,
	) -> Result<HashMap<L::Key, L::Value, S>, BatcherError<L::Error>> {
		self.batcher.execute_many(keys).await
	}
}

struct Wrapper<L: Loader<S>, S: BuildHasher + Default = RandomState>(L, PhantomData<S>);

impl<L: Loader<S>, S: BuildHasher + Default + Send + Sync> BatchOperation for Wrapper<L, S> {
	type Error = L::Error;
	type Item = L::Key;
	type Mode = BatcherDataloader<S>;
	type Response = L::Value;

	fn config(&self) -> BatcherConfig {
		self.0.config()
	}

	fn process(
		&self,
		documents: <Self::Mode as super::BatchMode<Self>>::Input,
	) -> impl std::future::Future<Output = Result<<Self::Mode as super::BatchMode<Self>>::OperationOutput, Self::Error>> + Send + '_ where Self: Send + Sync
	{
		async move { self.0.load(documents.into_iter().collect()).await }
	}
}

#[cfg(test)]
mod tests {
	use std::collections::hash_map::RandomState;
	use std::collections::HashMap;
	use std::convert::Infallible;

	use super::{DataLoader, LoaderOutput};
	use crate::batcher::BatcherConfig;

	type DynBoxLoader<S = RandomState> = Box<dyn Fn(Vec<u64>) -> HashMap<u64, u64, S> + Sync + Send>;

	struct LoaderTest {
		results: DynBoxLoader,
		config: BatcherConfig,
	}

	impl super::Loader for LoaderTest {
		type Error = Infallible;
		type Key = u64;
		type Value = u64;

		fn config(&self) -> BatcherConfig {
			self.config.clone()
		}

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

				assert_eq!(run_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst), 0);

				assert_eq!(keys.len(), 250);

				for key in keys {
					assert!(!results.contains_key(&key));

					results.insert(key, key * 2);
				}

				results
			}),
			config: BatcherConfig {
                name: "test".to_string(),
				concurrency: 10,
				max_batch_size: 1000,
				sleep_duration: std::time::Duration::from_millis(5),
			},
		};

		let dataloader = DataLoader::new(loader);

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
			config: BatcherConfig {
                name: "test".to_string(),
				concurrency: 10,
				max_batch_size: 1000,
				sleep_duration: std::time::Duration::from_millis(5),
			},
		};

		let dataloader = DataLoader::new(loader);

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
			config: BatcherConfig {
                name: "test".to_string(),
				concurrency: 10,
				max_batch_size: 3000,
				sleep_duration: std::time::Duration::from_millis(5),
			},
		};

		let dataloader = DataLoader::new(loader);

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
			config: BatcherConfig {
                name: "test".to_string(),
				concurrency: 10,
				max_batch_size: 1000,
				sleep_duration: std::time::Duration::from_millis(100),
			},
		};

		let dataloader = DataLoader::new(loader);

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
			config: BatcherConfig {
                name: "test".to_string(),
				concurrency: 10,
				max_batch_size: 1000,
				sleep_duration: std::time::Duration::from_millis(5),
			},
		};

		let dataloader = DataLoader::new(loader);

		let futures = vec![dataloader.load(5), dataloader.load(5), dataloader.load(5)];

		let results: Vec<_> = futures::future::join_all(futures).await;

		assert_eq!(results, vec![Ok(Some(10)), Ok(Some(10)), Ok(Some(10))]);
		assert_eq!(run_count.load(std::sync::atomic::Ordering::SeqCst), 1); // Ensure the loader was only called once
	}
}
