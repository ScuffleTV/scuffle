use std::collections::HashMap;
use std::sync::Arc;

use common::dataloader::{DataLoader, Loader, LoaderOutput};
use ulid::Ulid;
use uuid::Uuid;

use crate::database::Role;

pub struct RoleByIdLoader {
	db: Arc<sqlx::PgPool>,
}

impl RoleByIdLoader {
	pub fn new(db: Arc<sqlx::PgPool>) -> DataLoader<Self> {
		DataLoader::new(Self { db })
	}
}

#[async_trait::async_trait]
impl Loader for RoleByIdLoader {
	type Error = ();
	type Key = Ulid;
	type Value = Role;

	async fn load(&self, keys: &[Self::Key]) -> LoaderOutput<Self> {
		let results: Vec<Self::Value> = sqlx::query_as("SELECT * FROM roles WHERE id = ANY($1)")
			.bind(keys.iter().copied().map(Uuid::from).collect::<Vec<_>>())
			.fetch_all(self.db.as_ref())
			.await
			.map_err(|e| {
				tracing::error!(err = %e, "failed to fetch roles");
			})?;

		let mut map = HashMap::new();

		for result in results {
			map.insert(result.id.0, result);
		}

		Ok(map)
	}
}
