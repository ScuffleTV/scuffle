use std::sync::Arc;

use ulid::Ulid;
use utils::dataloader::{DataLoader, Loader, LoaderOutput};

use crate::database::Category;

pub struct CategoryByIdLoader {
	db: Arc<utils::database::Pool>,
}

impl CategoryByIdLoader {
	pub fn new(db: Arc<utils::database::Pool>) -> DataLoader<Self> {
		DataLoader::new(Self { db })
	}
}

impl Loader for CategoryByIdLoader {
	type Error = ();
	type Key = Ulid;
	type Value = Category;

	async fn load(&self, keys: &[Self::Key]) -> LoaderOutput<Self> {
		let results: Vec<Self::Value> = utils::database::query("SELECT * FROM categories WHERE id = ANY($1)")
			.bind(keys)
			.build_query_as()
			.fetch_all(&self.db)
			.await
			.map_err(|e| {
				tracing::error!(err = %e, "failed to fetch categories by id");
			})?;

		Ok(results.into_iter().map(|r| (r.id, r)).collect())
	}
}
