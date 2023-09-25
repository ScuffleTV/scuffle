use std::collections::hash_map::RandomState;

use crate::dataloader::LoaderOutput;

use super::Loader;

pub trait Cache<L: Loader<S>, S = RandomState> {
    fn contains_key(&self, key: &L::Key) -> bool;
    fn get(&self, key: &L::Key) -> Option<L::Value>;
    fn insert(&mut self, key: &L::Key, value: &L::Value);
    fn clear(&mut self) {}
    fn len(&self) -> usize;
    fn delete(&mut self, key: &L::Key) -> Option<L::Value>;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// # Safety
///
/// This trait is marked as unsafe because the implementor must ensure that the Cache is safe for concurrent access.
/// This will almost always be with some kind of interior mutability. Such as a `RwLock` or `Mutex`. Or if the cache performs no-ops on mutation.
/// Look at `SharedCache` for an example.
pub unsafe trait AutoImplCacheRef<L: Loader<S>, S = RandomState>:
    AutoImplCacheMutRef<L, S>
{
}

#[inline(always)]
#[allow(clippy::mut_from_ref)]
#[allow(invalid_reference_casting)]
fn upcast<T: AutoImplCacheRef<L, S>, L: Loader<S>, S>(cache: &T) -> impl Cache<L, S> {
    // Safety:
    // This is safe because the trait `AutoImplCacheRef` is marked as unsafe and therefore the implementor must ensure that the
    // Cache is safe for concurrent access. This is used to implement cache for &T where T is a Cache.
    // The issue is we need to upcast the reference to a mutable reference, even though the implementor only ever requires a reference.
    // This is not safe unless T has some kind of interior mutability.
    unsafe { &mut *(cache as *const T as *mut T) }
}

impl<L: Loader<S>, S, T: AutoImplCacheRef<L, S>> Cache<L, S> for &T {
    #[inline(always)]
    fn contains_key(&self, key: &L::Key) -> bool {
        (**self).contains_key(key)
    }

    #[inline(always)]
    fn get(&self, key: &L::Key) -> Option<L::Value> {
        (**self).get(key)
    }

    #[inline(always)]
    fn insert(&mut self, key: &L::Key, value: &L::Value) {
        upcast(*self).insert(key, value)
    }

    #[inline(always)]
    fn clear(&mut self) {
        upcast(*self).clear()
    }

    #[inline(always)]
    fn len(&self) -> usize {
        (**self).len()
    }

    #[inline(always)]
    fn delete(&mut self, key: &L::Key) -> Option<L::Value> {
        upcast(*self).delete(key)
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        (**self).is_empty()
    }
}

pub trait AutoImplCacheMutRef<L: Loader<S>, S = RandomState>: Cache<L, S> {}

impl<L: Loader<S>, S, T: AutoImplCacheMutRef<L, S>> Cache<L, S> for &mut T {
    #[inline(always)]
    fn contains_key(&self, key: &L::Key) -> bool {
        (**self).contains_key(key)
    }

    #[inline(always)]
    fn get(&self, key: &L::Key) -> Option<L::Value> {
        (**self).get(key)
    }

    #[inline(always)]
    fn insert(&mut self, key: &L::Key, value: &L::Value) {
        (**self).insert(key, value)
    }

    #[inline(always)]
    fn clear(&mut self) {
        (**self).clear()
    }

    #[inline(always)]
    fn len(&self) -> usize {
        (**self).len()
    }

    #[inline(always)]
    fn delete(&mut self, key: &L::Key) -> Option<L::Value> {
        (**self).delete(key)
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        (**self).is_empty()
    }
}

#[derive(Default, Clone, Debug, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoCache;

/// Safety: The no cache is always for safe for concurrent access because it is a no-op cache and therefore has no internal state.
unsafe impl<L: Loader<S>, S> AutoImplCacheRef<L, S> for NoCache {}

impl<L: Loader<S>, S> AutoImplCacheMutRef<L, S> for NoCache {}

impl<L: Loader<S>, S> Cache<L, S> for NoCache {
    #[inline(always)]
    fn contains_key(&self, _: &L::Key) -> bool {
        false
    }

    #[inline(always)]
    fn get(&self, _: &L::Key) -> Option<L::Value> {
        None
    }

    #[inline(always)]
    fn insert(&mut self, _: &L::Key, _: &L::Value) {}

    #[inline(always)]
    fn clear(&mut self) {}

    #[inline(always)]
    fn len(&self) -> usize {
        0
    }

    #[inline(always)]
    fn delete(&mut self, _: &L::Key) -> Option<L::Value> {
        None
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        true
    }
}

#[derive(Clone, Debug)]
pub struct HashMapCache<L: Loader<S>, S = RandomState, S2 = S>(
    std::collections::HashMap<L::Key, L::Value, S2>,
);

impl<L: Loader<S>, S, S2: std::hash::BuildHasher> AutoImplCacheMutRef<L, S>
    for HashMapCache<L, S, S2>
{
}

impl<L: Loader<S>, S, S2: std::hash::BuildHasher> Cache<L, S> for HashMapCache<L, S, S2> {
    #[inline(always)]
    fn contains_key(&self, key: &L::Key) -> bool {
        self.0.contains_key(key)
    }

    #[inline(always)]
    fn get(&self, key: &L::Key) -> Option<L::Value> {
        self.0.get(key).cloned()
    }

    #[inline(always)]
    fn insert(&mut self, key: &L::Key, value: &L::Value) {
        self.0.insert(key.clone(), value.clone());
    }

    #[inline(always)]
    fn clear(&mut self) {
        self.0.clear();
    }

    #[inline(always)]
    fn len(&self) -> usize {
        self.0.len()
    }

    #[inline(always)]
    fn delete(&mut self, key: &L::Key) -> Option<L::Value> {
        self.0.remove(key)
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<L: Loader<S>, S: std::hash::BuildHasher + Default> Default for HashMapCache<L, S> {
    #[inline(always)]
    fn default() -> Self {
        Self(std::collections::HashMap::default())
    }
}

#[derive(Debug)]
pub struct SharedCache<C>(std::sync::Arc<std::sync::RwLock<C>>);

impl<C> Clone for SharedCache<C> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<C> SharedCache<C> {
    #[inline(always)]
    pub fn new(cache: C) -> Self {
        Self(std::sync::Arc::new(std::sync::RwLock::new(cache)))
    }
}

impl<C: Default> Default for SharedCache<C> {
    #[inline(always)]
    fn default() -> Self {
        Self::new(C::default())
    }
}

impl<C: Cache<L, S>, L: Loader<S>, S> AutoImplCacheMutRef<L, S> for SharedCache<C> {}
/// Safety: The shared cache is always for concurrent access because it contains a RwLock, which is a safe concurrent access primitive.
unsafe impl<'a, C: Cache<L, S> + 'a, L: Loader<S>, S> AutoImplCacheRef<L, S> for SharedCache<C> {}

impl<C: Cache<L, S>, L: Loader<S>, S> Cache<L, S> for SharedCache<C> {
    #[inline(always)]
    fn contains_key(&self, key: &L::Key) -> bool {
        self.0.read().unwrap().contains_key(key)
    }

    #[inline(always)]
    fn get(&self, key: &L::Key) -> Option<L::Value> {
        self.0.read().unwrap().get(key)
    }

    #[inline(always)]
    fn insert(&mut self, key: &L::Key, value: &L::Value) {
        self.0.write().unwrap().insert(key, value)
    }

    #[inline(always)]
    fn clear(&mut self) {
        self.0.write().unwrap().clear()
    }

    #[inline(always)]
    fn len(&self) -> usize {
        self.0.read().unwrap().len()
    }

    #[inline(always)]
    fn delete(&mut self, key: &L::Key) -> Option<L::Value> {
        self.0.write().unwrap().delete(key)
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        self.0.read().unwrap().is_empty()
    }
}

// This is a compile time test to ensure that the traits are implemented correctly.
const _: () = {
    struct DummyLoader;

    #[async_trait::async_trait]
    impl<S> Loader<S> for DummyLoader {
        type Key = ();
        type Value = ();
        type Error = ();

        async fn load(&self, _: &[Self::Key]) -> LoaderOutput<Self, S> {
            unimplemented!()
        }
    }

    const fn assert_auto_impl_cache_ref<C: Cache<L, S>, L: Loader<S>, S>() {}

    assert_auto_impl_cache_ref::<NoCache, DummyLoader, RandomState>();
    assert_auto_impl_cache_ref::<&NoCache, DummyLoader, RandomState>();
    assert_auto_impl_cache_ref::<&mut NoCache, DummyLoader, RandomState>();

    assert_auto_impl_cache_ref::<HashMapCache<DummyLoader>, DummyLoader, RandomState>();
    assert_auto_impl_cache_ref::<&mut HashMapCache<DummyLoader>, DummyLoader, RandomState>();

    assert_auto_impl_cache_ref::<SharedCache<NoCache>, DummyLoader, RandomState>();
    assert_auto_impl_cache_ref::<&SharedCache<NoCache>, DummyLoader, RandomState>();
    assert_auto_impl_cache_ref::<&mut SharedCache<NoCache>, DummyLoader, RandomState>();

    assert_auto_impl_cache_ref::<SharedCache<HashMapCache<DummyLoader>>, DummyLoader, RandomState>(
    );
    assert_auto_impl_cache_ref::<&SharedCache<HashMapCache<DummyLoader>>, DummyLoader, RandomState>(
    );
    assert_auto_impl_cache_ref::<
        &mut SharedCache<HashMapCache<DummyLoader>>,
        DummyLoader,
        RandomState,
    >();

    assert_auto_impl_cache_ref::<
        SharedCache<SharedCache<HashMapCache<DummyLoader>>>,
        DummyLoader,
        RandomState,
    >();
    assert_auto_impl_cache_ref::<
        &SharedCache<SharedCache<HashMapCache<DummyLoader>>>,
        DummyLoader,
        RandomState,
    >();
    assert_auto_impl_cache_ref::<
        &mut SharedCache<SharedCache<HashMapCache<DummyLoader>>>,
        DummyLoader,
        RandomState,
    >();

    assert_auto_impl_cache_ref::<
        SharedCache<&SharedCache<HashMapCache<DummyLoader>>>,
        DummyLoader,
        RandomState,
    >();
    assert_auto_impl_cache_ref::<
        &SharedCache<&SharedCache<HashMapCache<DummyLoader>>>,
        DummyLoader,
        RandomState,
    >();
    assert_auto_impl_cache_ref::<
        &mut SharedCache<&SharedCache<HashMapCache<DummyLoader>>>,
        DummyLoader,
        RandomState,
    >();
};
