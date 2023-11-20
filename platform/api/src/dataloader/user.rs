use std::collections::HashMap;
use std::sync::Arc;

use common::dataloader::{DataLoader, Loader, LoaderOutput};
use ulid::Ulid;
use uuid::Uuid;

use crate::database::User;

pub struct UserByUsernameLoader {
	db: Arc<sqlx::PgPool>,
}

impl UserByUsernameLoader {
	pub fn new(db: Arc<sqlx::PgPool>) -> DataLoader<Self> {
		DataLoader::new(Self { db })
	}
}

#[async_trait::async_trait]
impl Loader for UserByUsernameLoader {
	type Error = ();
	type Key = String;
	type Value = User;

	async fn load(&self, keys: &[Self::Key]) -> LoaderOutput<Self> {
		let results: Vec<Self::Value> = sqlx::query_as("SELECT * FROM users WHERE username = ANY($1)")
			.bind(keys)
			.fetch_all(self.db.as_ref())
			.await
			.map_err(|e| {
				tracing::error!(err = %e, "failed to fetch users by username");
			})?;

		let mut map = HashMap::new();

		for result in results {
			map.insert(result.username.clone(), result);
		}

		Ok(map)
	}
}

pub struct UserByIdLoader {
	db: Arc<sqlx::PgPool>,
}

impl UserByIdLoader {
	pub fn new(db: Arc<sqlx::PgPool>) -> DataLoader<Self> {
		DataLoader::new(Self { db })
	}
}

#[async_trait::async_trait]
impl Loader for UserByIdLoader {
	type Error = ();
	type Key = Ulid;
	type Value = User;

	async fn load(&self, keys: &[Self::Key]) -> LoaderOutput<Self> {
		let results: Vec<Self::Value> = sqlx::query_as("SELECT * FROM users WHERE id = ANY($1)")
			.bind(keys.iter().copied().map(Uuid::from).collect::<Vec<_>>())
			.fetch_all(self.db.as_ref())
			.await
			.map_err(|e| {
				tracing::error!(err = %e, "failed to fetch users by id");
			})?;

		let mut map = HashMap::new();

		for result in results {
			map.insert(result.id.0, result);
		}

		Ok(map)
	}
}
