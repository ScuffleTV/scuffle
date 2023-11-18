use std::sync::Arc;

use common::dataloader::{DataLoader, Loader, LoaderOutput};
use ulid::Ulid;
use uuid::Uuid;

pub struct AccessTokenLoader {
	db: Arc<sqlx::PgPool>,
}

impl AccessTokenLoader {
	pub fn new(db: Arc<sqlx::PgPool>) -> DataLoader<Self> {
		DataLoader::new(Self { db })
	}
}

#[async_trait::async_trait]
impl Loader for AccessTokenLoader {
	type Error = ();
	type Key = Ulid;
	type Value = video_common::database::AccessToken;

	async fn load(&self, key: &[Self::Key]) -> LoaderOutput<Self> {
		let results: Vec<Self::Value> = sqlx::query_as(
			r#"
            SELECT * FROM access_tokens
            WHERE id = ANY($1)
            "#,
		)
		.bind(key.iter().copied().map(Uuid::from).collect::<Vec<_>>())
		.fetch_all(self.db.as_ref())
		.await
		.map_err(|err| {
			tracing::error!(error = %err, "failed to load access tokens");
		})?;

		Ok(results.into_iter().map(|v| (v.id.0, v)).collect())
	}
}
