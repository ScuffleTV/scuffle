use std::{
    collections::{hash_map::RandomState, HashMap},
    sync::Arc,
};

use tokio::sync::{Mutex, RwLock};

use super::{batch_loader::BatchLoader, Loader};

pub(super) struct DataLoaderInner<L: Loader<S>, S = RandomState> {
    pub active_batch: Option<BatchLoader<L, S>>,
    pub semaphore: Arc<tokio::sync::Semaphore>,
}

#[allow(type_alias_bounds)]
pub type LoaderOutput<L: Loader<S>, S = RandomState> =
    Result<HashMap<L::Key, L::Value, S>, L::Error>;

#[allow(type_alias_bounds)]
pub(super) type BatchState<L: Loader<S>, S = RandomState> = (
    Arc<RwLock<Option<LoaderOutput<L, S>>>>,
    tokio_util::sync::WaitForCancellationFutureOwned,
);

#[allow(type_alias_bounds)]
pub(super) type DataLoaderInnerHolder<L: Loader<S>, S = RandomState> =
    Arc<Mutex<DataLoaderInner<L, S>>>;
