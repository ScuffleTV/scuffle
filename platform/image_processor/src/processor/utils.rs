use std::sync::Arc;

use ulid::Ulid;

use super::error::ProcessorError;
use crate::database::Job;
use crate::global::ImageProcessorGlobal;
use crate::processor::error::Result;

pub async fn query_job(global: &Arc<impl ImageProcessorGlobal>, limit: usize) -> Result<Vec<Job>> {
	Ok(common::database::query(
		"UPDATE image_jobs
		SET claimed_by = $1,
			hold_until = NOW() + INTERVAL '30 seconds'
		FROM (
			SELECT id
			FROM image_jobs
			WHERE hold_until IS NULL OR hold_until < NOW()
			ORDER BY priority DESC,
				id DESC
			LIMIT $2
		) AS job
		WHERE image_jobs.id = job.id
		RETURNING image_jobs.id, image_jobs.task",
	)
	.bind(global.config().instance_id)
	.bind(limit as i64)
	.build_query_as()
	.fetch_all(global.db())
	.await?)
}

pub async fn refresh_job(global: &Arc<impl ImageProcessorGlobal>, job_id: Ulid) -> Result<()> {
	let result = common::database::query(
		"UPDATE image_jobs
		SET hold_until = NOW() + INTERVAL '30 seconds'
		WHERE image_jobs.id = $1 AND image_jobs.claimed_by = $2",
	)
	.bind(job_id)
	.bind(global.config().instance_id)
	.build()
	.execute(global.db())
	.await?;

	if result == 0 { Err(ProcessorError::LostJob) } else { Ok(()) }
}

pub async fn delete_job(global: &Arc<impl ImageProcessorGlobal>, job_id: Ulid) -> Result<()> {
	common::database::query("DELETE FROM image_jobs WHERE id = $1")
		.bind(job_id)
		.build()
		.execute(global.db())
		.await?;

	Ok(())
}
