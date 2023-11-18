use std::collections::HashMap;
use std::sync::Arc;

use common::dataloader::{DataLoader, Loader, LoaderOutput};

use crate::database::GlobalState;

pub struct GlobalStateLoader {
	db: Arc<sqlx::PgPool>,
}

impl GlobalStateLoader {
	pub fn new(db: Arc<sqlx::PgPool>) -> DataLoader<Self> {
		DataLoader::new(Self { db })
	}
}

#[async_trait::async_trait]
impl Loader for GlobalStateLoader {
	type Error = ();
	type Key = ();
	type Value = GlobalState;

	async fn load(&self, _: &[Self::Key]) -> LoaderOutput<Self> {
		let state = sqlx::query_as("SELECT * FROM global_state")
			.fetch_one(self.db.as_ref())
			.await
			.map_err(|e| {
				tracing::error!(err = %e, "failed to fetch global state");
			})?;

		let mut map = HashMap::new();
		map.insert((), state);

		Ok(map)
	}
}
