use std::sync::Arc;

use common::dataloader::{DataLoader, Loader, LoaderOutput};
use ulid::Ulid;

pub struct AccessTokenLoader {
	db: Arc<sqlx::PgPool>,
}

impl AccessTokenLoader {
	pub fn new(db: Arc<sqlx::PgPool>) -> DataLoader<Self> {
		DataLoader::new(Self { db })
	}
}

impl Loader for AccessTokenLoader {
	type Error = ();
	type Key = (Ulid, Ulid);
	type Value = video_common::database::AccessToken;

	async fn load(&self, key: &[Self::Key]) -> LoaderOutput<Self> {
		let ids = key.iter().copied().map(|(organization_id, access_token_id)| {
			(
				common::database::Ulid(organization_id),
				common::database::Ulid(access_token_id),
			)
		});

		let mut qb = sqlx::query_builder::QueryBuilder::default();

		qb.push("SELECT * FROM access_tokens WHERE (organization_id, id) IN ");

		qb.push_tuples(ids, |mut qb, (organization_id, access_token_id)| {
			qb.push_bind(organization_id).push_bind(access_token_id);
		});

		let results: Vec<Self::Value> = qb.build_query_as().fetch_all(self.db.as_ref()).await.map_err(|err| {
			tracing::error!(error = %err, "failed to load access tokens");
		})?;

		Ok(results.into_iter().map(|v| ((v.organization_id.0, v.id.0), v)).collect())
	}
}
