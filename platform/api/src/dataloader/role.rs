use std::sync::Arc;

use ulid::Ulid;
use utils::dataloader::{DataLoader, Loader, LoaderOutput};

use crate::database::Role;

pub struct RoleByIdLoader {
	db: Arc<utils::database::Pool>,
}

impl RoleByIdLoader {
	pub fn new(db: Arc<utils::database::Pool>) -> DataLoader<Self> {
		DataLoader::new(Self { db })
	}
}

impl Loader for RoleByIdLoader {
	type Error = ();
	type Key = Ulid;
	type Value = Role;

	async fn load(&self, keys: &[Self::Key]) -> LoaderOutput<Self> {
		let results: Vec<Self::Value> = utils::database::query("SELECT * FROM roles WHERE id = ANY($1)")
			.bind(keys)
			.build_query_as()
			.fetch_all(self.db.as_ref())
			.await
			.map_err(|e| {
				tracing::error!(err = %e, "failed to fetch roles");
			})?;

		Ok(results.into_iter().map(|r| (r.id, r)).collect())
	}
}
