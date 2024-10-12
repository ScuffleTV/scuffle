use std::collections::HashMap;
use std::sync::Arc;

use scuffle_utilsdataloader::{DataLoader, Loader, LoaderOutput};

use crate::database::GlobalState;

pub struct GlobalStateLoader {
	db: Arc<utils::database::Pool>,
}

impl GlobalStateLoader {
	pub fn new(db: Arc<utils::database::Pool>) -> DataLoader<Self> {
		DataLoader::new(Self { db })
	}
}

impl Loader for GlobalStateLoader {
	type Error = ();
	type Key = ();
	type Value = GlobalState;

	async fn load(&self, _: &[Self::Key]) -> LoaderOutput<Self> {
		let state = scuffle_utils::database::query("SELECT * FROM global_state")
			.build_query_as()
			.fetch_one(&self.db)
			.await
			.map_err(|e| {
				tracing::error!(err = %e, "failed to fetch global state");
			})?;

		let mut map = HashMap::new();
		map.insert((), state);

		Ok(map)
	}
}
