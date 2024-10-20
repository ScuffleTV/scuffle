use std::sync::Arc;

use scuffle_utilsdataloader::{DataLoader, Loader, LoaderOutput};
use ulid::Ulid;

use crate::database::UploadedFile;

pub struct UploadedFileByIdLoader {
	db: Arc<utils::database::Pool>,
}

impl UploadedFileByIdLoader {
	pub fn new(db: Arc<utils::database::Pool>) -> DataLoader<Self> {
		DataLoader::new(Self { db })
	}
}

impl Loader for UploadedFileByIdLoader {
	type Error = ();
	type Key = Ulid;
	type Value = UploadedFile;

	async fn load(&self, keys: &[Self::Key]) -> LoaderOutput<Self> {
		let results: Vec<Self::Value> = scuffle_utils::database::query("SELECT * FROM uploaded_files WHERE id = ANY($1)")
			.bind(keys)
			.build_query_as()
			.fetch_all(self.db.as_ref())
			.await
			.map_err(|e| {
				tracing::error!(err = %e, "failed to fetch users by username");
			})?;

		Ok(results.into_iter().map(|r| (r.id, r)).collect())
	}
}
