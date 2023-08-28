use std::{
    collections::{hash_map::RandomState, HashSet},
    sync::Arc,
};

use tokio::sync::RwLock;

use super::{Loader, LoaderOutput};

pub(super) struct BatchLoader<L: Loader<S>, S = RandomState> {
    pub id: u64,
    pub loader: Arc<L>,
    pub keys: HashSet<L::Key, S>,
    pub start: tokio::time::Instant,
    pub result: Arc<RwLock<Option<LoaderOutput<L, S>>>>,
    pub token: tokio_util::sync::CancellationToken,
}

impl<L: Loader<S>, S> BatchLoader<L, S> {
    pub async fn load(self, sephamore: Arc<tokio::sync::Semaphore>) {
        let _ticket = sephamore.acquire().await.unwrap();
        let _drop = self.token.drop_guard();
        let keys = self.keys.iter().cloned().collect::<Vec<_>>();
        *(self.result.write().await) = Some(self.loader.load(&keys).await);
    }
}
