use std::sync::Arc;

use common::dataloader::{DataLoader, Loader, LoaderOutput};
use ulid::Ulid;

use crate::database::User;

pub struct UserByUsernameLoader {
	db: Arc<common::database::Pool>,
}

impl UserByUsernameLoader {
	pub fn new(db: Arc<common::database::Pool>) -> DataLoader<Self> {
		DataLoader::new(Self { db })
	}
}

impl Loader for UserByUsernameLoader {
	type Error = ();
	type Key = String;
	type Value = User;

	async fn load(&self, keys: &[Self::Key]) -> LoaderOutput<Self> {
		let results: Vec<Self::Value> = common::database::query("SELECT * FROM users WHERE username = ANY($1)")
			.bind(keys)
			.build_query_as()
			.fetch_all(self.db.as_ref())
			.await
			.map_err(|e| {
				tracing::error!(err = %e, "failed to fetch users by username");
			})?;

		Ok(results.into_iter().map(|r| (r.username.clone(), r)).collect())
	}
}

pub struct UserByIdLoader {
	db: Arc<common::database::Pool>,
}

impl UserByIdLoader {
	pub fn new(db: Arc<common::database::Pool>) -> DataLoader<Self> {
		DataLoader::new(Self { db })
	}
}

impl Loader for UserByIdLoader {
	type Error = ();
	type Key = Ulid;
	type Value = User;

	async fn load(&self, keys: &[Self::Key]) -> LoaderOutput<Self> {
		let results: Vec<Self::Value> = common::database::query("SELECT * FROM users WHERE id = ANY($1)")
			.bind(keys)
			.build_query_as()
			.fetch_all(self.db.as_ref())
			.await
			.map_err(|e| {
				tracing::error!(err = %e, "failed to fetch users by id");
			})?;

		Ok(results.into_iter().map(|r| (r.id, r)).collect())
	}
}
