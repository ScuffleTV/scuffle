use std::sync::Arc;

use common::dataloader::{DataLoader, Loader, LoaderOutput};
use ulid::Ulid;

pub struct RoomLoader {
	db: Arc<sqlx::PgPool>,
}

impl RoomLoader {
	pub fn new(db: Arc<sqlx::PgPool>) -> DataLoader<Self> {
		DataLoader::new(Self { db })
	}
}

#[async_trait::async_trait]
impl Loader for RoomLoader {
	type Error = ();
	type Key = (Ulid, Ulid);
	type Value = video_common::database::Room;

	async fn load(&self, key: &[Self::Key]) -> LoaderOutput<Self> {
		let ids = key
			.iter()
			.copied()
			.map(|(organization_id, room_id)| (common::database::Ulid(organization_id), common::database::Ulid(room_id)));

		let mut qb = sqlx::query_builder::QueryBuilder::default();

		qb.push("SELECT * FROM rooms WHERE (organization_id, id) IN ");
		qb.push_tuples(ids, |mut qb, (organization_id, room_id)| {
			qb.push_bind(organization_id).push_bind(room_id);
		});

		let results: Vec<Self::Value> = qb.build_query_as().fetch_all(self.db.as_ref()).await.map_err(|err| {
			tracing::error!(error = %err, "failed to load rooms");
		})?;

		Ok(results.into_iter().map(|v| ((v.organization_id.0, v.id.0), v)).collect())
	}
}
