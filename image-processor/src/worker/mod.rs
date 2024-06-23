use std::sync::Arc;

use anyhow::Context;
use scuffle_foundations::context::{self, ContextFutExt};

use crate::database::Job;
use crate::global::Global;

pub mod process;

pub use self::process::JobError;

pub async fn start(global: Arc<Global>) -> anyhow::Result<()> {
	let config = global.config();

	let mut concurrency = config.worker.concurrency;

	if concurrency == 0 {
		concurrency = num_cpus::get();
	}

	let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));

	let mut error_count = 0;
	let (_, handle) = context::Context::new();

	loop {
		let ctx = handle.context();
		let Some(permit) = semaphore
			.clone()
			.acquire_owned()
			.with_context(&ctx)
			.await
			.transpose()
			.expect("semaphore permit")
		else {
			break;
		};

		let job = match Job::fetch(&global).await {
			Ok(Some(job)) => job,
			Ok(None) => {
				tokio::time::sleep(config.worker.polling_interval).await;
				continue;
			}
			Err(err) => {
				tracing::error!("failed to fetch job: {err}");
				error_count += 1;
				if error_count >= config.worker.error_threshold {
					Err(err).context("reached error threshold")?;
				}

				tokio::time::sleep(config.worker.error_delay).await;

				continue;
			}
		};

		error_count = 0;
		tokio::spawn(self::process::spawn(job, global.clone(), ctx, permit));
	}

	handle.shutdown().await;

	Ok(())
}
