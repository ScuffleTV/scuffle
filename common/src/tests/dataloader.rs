use std::collections::hash_map::RandomState;
use std::collections::HashMap;

use crate::dataloader::{Cache, HashMapCache, LoaderOutput, NoCache, SharedCache};

type DynBoxLoader<S = RandomState> = Box<dyn Fn(&[u64]) -> HashMap<u64, u64, S> + Sync + Send>;

struct LoaderTest {
	results: DynBoxLoader,
}

impl crate::dataloader::Loader for LoaderTest {
	type Error = ();
	type Key = u64;
	type Value = u64;

	async fn load(&self, keys: &[Self::Key]) -> LoaderOutput<Self> {
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
				assert!(!results.contains_key(key));

				results.insert(*key, *key * 2);
			}

			results
		}),
	};

	let dataloader = crate::dataloader::DataLoader::new(loader);

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
				assert!(!results.contains_key(key));

				results.insert(*key, *key * 2);
			}

			results
		}),
	};

	let dataloader = crate::dataloader::DataLoader::new(loader);

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
				assert!(!results.contains_key(key));

				results.insert(*key, *key * 2);
			}

			results
		}),
	};

	let dataloader = crate::dataloader::DataLoader::new(loader).set_max_batch_size(3000);

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
				assert!(!results.contains_key(key));

				results.insert(*key, *key * 2);
			}

			results
		}),
	};

	let dataloader = crate::dataloader::DataLoader::new(loader).set_duration(tokio::time::Duration::from_millis(100));

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
async fn test_data_loader_no_cache() {
	let loader = LoaderTest {
		results: Box::new(|keys| keys.iter().map(|&key| (key, key * 2)).collect()),
	};

	let dataloader = crate::dataloader::DataLoader::new(loader);

	let result = dataloader.load_with_cache(NoCache, 5).await;

	assert_eq!(result, Ok(Some(10)));
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

	let dataloader = crate::dataloader::DataLoader::new(loader);

	let futures = vec![dataloader.load(5), dataloader.load(5), dataloader.load(5)];

	let results: Vec<_> = futures::future::join_all(futures).await;

	assert_eq!(results, vec![Ok(Some(10)), Ok(Some(10)), Ok(Some(10))]);
	assert_eq!(run_count.load(std::sync::atomic::Ordering::SeqCst), 1); // Ensure the loader was only called once
}

#[tokio::test]
async fn test_data_loader_hash_map_cache() {
	let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));

	let loader = LoaderTest {
		results: Box::new({
			let call_count = call_count.clone();
			move |keys| {
				call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
				keys.iter().map(|&key| (key, key * 2)).collect()
			}
		}),
	};

	let dataloader = crate::dataloader::DataLoader::new(loader);

	let mut cache = HashMapCache::default();
	let result1 = dataloader.load_with_cache(&mut cache, 5).await;
	let result2 = dataloader.load_with_cache(&mut cache, 5).await;

	assert_eq!(result1, Ok(Some(10)));
	assert_eq!(result2, Ok(Some(10))); // This should be fetched from the cache
	assert_eq!(cache.len(), 1); // Ensure the cache size is 1
	assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1); // Ensure the loader was only called once
}

#[tokio::test]
async fn test_data_loader_shared_cache() {
	let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));

	let loader = LoaderTest {
		results: Box::new({
			let call_count = call_count.clone();
			move |keys| {
				call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
				keys.iter().map(|&key| (key, key * 2)).collect()
			}
		}),
	};

	let dataloader = crate::dataloader::DataLoader::new(loader);

	let cache = SharedCache::new(HashMapCache::default());
	let result1 = dataloader.load_with_cache(cache.clone(), 5).await;
	let result2 = dataloader.load_with_cache(cache.clone(), 5).await;

	assert_eq!(result1, Ok(Some(10)));
	assert_eq!(result2, Ok(Some(10))); // This should be fetched from the cache
	assert_eq!(cache.len(), 1); // Ensure the cache size is 1
	assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1); // Ensure the loader was only called once
}

#[tokio::test]
async fn test_data_loader_shared_cache_ref() {
	let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));

	let loader = LoaderTest {
		results: Box::new({
			let call_count = call_count.clone();
			move |keys| {
				call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
				keys.iter().map(|&key| (key, key * 2)).collect()
			}
		}),
	};

	let dataloader = crate::dataloader::DataLoader::new(loader);

	let cache = SharedCache::new(HashMapCache::default());
	let result1 = dataloader.load_with_cache(&cache, 5).await;
	let result2 = dataloader.load_with_cache(&cache, 5).await;

	assert_eq!(result1, Ok(Some(10)));
	assert_eq!(result2, Ok(Some(10))); // This should be fetched from the cache
	assert_eq!(cache.len(), 1); // Ensure the cache size is 1
	assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1); // Ensure the loader was only called once
}
