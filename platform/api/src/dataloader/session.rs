use std::sync::Arc;

use common::dataloader::{DataLoader, Loader, LoaderOutput};
use ulid::Ulid;
use uuid::Uuid;

use crate::database::Session;

pub struct SessionByIdLoader {
	db: Arc<sqlx::PgPool>,
}

impl SessionByIdLoader {
	pub fn new(db: Arc<sqlx::PgPool>) -> DataLoader<Self> {
		DataLoader::new(Self { db })
	}
}

impl Loader for SessionByIdLoader {
	type Error = ();
	type Key = Ulid;
	type Value = Session;

	async fn load(&self, keys: &[Self::Key]) -> LoaderOutput<Self> {
		let results: Vec<Self::Value> = sqlx::query_as("SELECT * FROM user_sessions WHERE id = ANY($1)")
			.bind(keys.iter().copied().map(Uuid::from).collect::<Vec<_>>())
			.fetch_all(self.db.as_ref())
			.await
			.map_err(|e| {
				tracing::error!(err = %e, "failed to fetch sessions");
			})?;

		Ok(results.into_iter().map(|r| (r.id.0, r)).collect())
	}
}
