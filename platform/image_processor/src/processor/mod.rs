use std::sync::Arc;

use futures::StreamExt;
use tokio::select;

use self::error::Result;
use crate::config::ImageProcessorConfig;
use crate::global::ImageProcessorGlobal;
use crate::processor::error::ProcessorError;
use crate::processor::job::handle_job;

pub(crate) mod error;
pub(crate) mod job;
pub(crate) mod utils;

pub async fn run(global: Arc<impl ImageProcessorGlobal>) -> Result<()> {
	let config = global.config::<ImageProcessorConfig>();

	let semaphore = tokio::sync::Semaphore::new(config.concurrency);

	let mut done = global.ctx().done();

	let working_directory = if let Some(working_directory) = &config.working_directory {
		if working_directory.is_empty() || working_directory == "." {
			std::env::current_dir().ok()
		} else {
			Some(std::path::PathBuf::from(working_directory))
		}
	} else {
		None
	}
	.unwrap_or_else(|| std::path::PathBuf::from(format!("/tmp/{}", config.instance_id)));

	tokio::fs::create_dir_all(&working_directory)
		.await
		.map_err(ProcessorError::DirectoryCreate)?;
	std::env::set_current_dir(&working_directory).map_err(ProcessorError::WorkingDirectoryChange)?;

	let mut futures = futures::stream::FuturesUnordered::new();

	loop {
		select! {
			ticket_job = async {
				let ticket = semaphore.acquire().await?;

				if let Some(job) = utils::query_job(&global).await? {
					Ok::<_, ProcessorError>(Some((ticket, job)))
				} else {
					tokio::time::sleep(std::time::Duration::from_secs(1)).await;
					Ok::<_, ProcessorError>(None)
				}
			} => {
				let Some((ticket, job)) = ticket_job? else {
					continue;
				};

				futures.push(handle_job(&global, &working_directory, ticket, job));
			},
			Some(r) = futures.next() => {
				r?;
			},
			_ = &mut done => break,
		}
	}

	Ok(())
}
