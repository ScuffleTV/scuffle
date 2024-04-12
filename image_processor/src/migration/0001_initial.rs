use std::sync::Arc;

use utils::database::deadpool_postgres::Transaction;

use super::Migration;
use crate::global::ImageProcessorGlobal;

pub struct InitialMigration;

#[async_trait::async_trait]
impl<S: ImageProcessorGlobal> Migration<S> for InitialMigration {
	fn name(&self) -> &'static str {
		"InitialMigration"
	}

	fn version(&self) -> i32 {
		1
	}

	async fn up(&self, _: &Arc<S>, tx: &Transaction<'_>) -> anyhow::Result<()> {
		utils::database::query(
			"CREATE TABLE image_processor_job (
                id UUID PRIMARY KEY,
                hold_until TIMESTAMP WITH TIME ZONE,
                priority INTEGER NOT NULL,
                claimed_by_id UUID,
                task bytea NOT NULL
            );",
		)
		.build()
		.execute(tx)
		.await?;

		utils::database::query("CREATE INDEX image_processor_job_hold_until_index ON image_processor_job (hold_until ASC);")
			.build()
			.execute(tx)
			.await?;

		utils::database::query(
			"CREATE INDEX image_processor_job_priority_index ON image_processor_job (priority DESC, id DESC);",
		)
		.build()
		.execute(tx)
		.await?;

		Ok(())
	}

	async fn down(&self, _: &Arc<S>, tx: &Transaction<'_>) -> anyhow::Result<()> {
		utils::database::query("DROP TABLE image_jobs").build().execute(tx).await?;

		Ok(())
	}
}
