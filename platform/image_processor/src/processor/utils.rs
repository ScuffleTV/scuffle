use std::sync::Arc;

use ulid::Ulid;

use super::error::ProcessorError;
use crate::database::Job;
use crate::global::ImageProcessorGlobal;
use crate::processor::error::Result;

pub async fn query_job(global: &Arc<impl ImageProcessorGlobal>) -> Result<Option<Job>> {
	Ok(sqlx::query_as(
		"UPDATE image_jobs
		SET claimed_by = $1,
			hold_until = NOW() + INTERVAL '30 seconds'
		FROM (
			SELECT id
			FROM image_jobs
			WHERE hold_until IS NULL OR hold_until < NOW()
			ORDER BY priority DESC,
				id DESC
			LIMIT 1
		) AS job
		WHERE image_jobs.id = job.id
		RETURNING image_jobs.id, image_jobs.task",
	)
	.bind(common::database::Ulid(global.config().instance_id))
	.fetch_optional(global.db().as_ref())
	.await?)
}

pub async fn refresh_job(global: &Arc<impl ImageProcessorGlobal>, job_id: Ulid) -> Result<()> {
	let result = sqlx::query(
		"UPDATE image_jobs
		SET hold_until = NOW() + INTERVAL '30 seconds'
		WHERE image_jobs.id = $1 AND image_jobs.claimed_by = $2",
	)
	.bind(common::database::Ulid(job_id))
	.bind(common::database::Ulid(global.config().instance_id))
	.execute(global.db().as_ref())
	.await?;

	if result.rows_affected() == 0 {
		Err(ProcessorError::LostJob)
	} else {
		Ok(())
	}
}
