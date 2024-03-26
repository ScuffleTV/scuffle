use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use futures::StreamExt;
use tokio::select;

use self::error::Result;
use crate::config::ImageProcessorConfig;
use crate::global::ImageProcessorGlobal;
use crate::processor::job::handle_job;

pub(crate) mod error;
pub(crate) mod job;
pub(crate) mod utils;

pub async fn run(global: Arc<impl ImageProcessorGlobal>) -> Result<()> {
	let config = global.config::<ImageProcessorConfig>();

	let concurrency = AtomicUsize::new(config.concurrency);

	let mut done = global.ctx().done();

	let mut futures = futures::stream::FuturesUnordered::new();

	let make_job_query = {
		let global = &global;
		let concurrency = &concurrency;
		move |wait: bool| async move {
			if wait {
				tokio::time::sleep(std::time::Duration::from_secs(1)).await;
			}

			let concurrency = concurrency.load(std::sync::atomic::Ordering::Relaxed);

			if concurrency == 0 {
				tracing::debug!("concurrency limit reached, waiting for a slot");
				None
			} else {
				tracing::debug!("querying for jobs: {concurrency}");
				Some(utils::query_job(global, concurrency).await)
			}
		}
	};

	let mut job_query = Some(Box::pin(make_job_query(false)));

	loop {
		select! {
			Some(jobs) = async {
				if let Some(job_query_fut) = &mut job_query {
					let r = job_query_fut.await;
					job_query = None;
					r
				} else {
					None
				}
			}  => {
				let jobs = jobs?;
				tracing::debug!("got {} jobs", jobs.len());
				job_query = Some(Box::pin(make_job_query(jobs.is_empty())));

				for job in jobs {
					concurrency.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
					futures.push(handle_job(&global, job));
				}
			},
			Some(_) = futures.next() => {
				concurrency.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
				if job_query.is_none() {
					job_query = Some(Box::pin(make_job_query(true)));
				}
			},
			_ = &mut done => break,
		}
	}

	Ok(())
}
