use anyhow::Context;
use futures_util::Future;
use tokio::task::JoinHandle;

pub mod generic;
pub mod recording;
pub mod rendition;
pub mod track_parser;

pub struct Task {
	handle: Option<JoinHandle<anyhow::Result<()>>>,
	tag: String,
}

impl Task {
	pub fn new(handle: JoinHandle<anyhow::Result<()>>, tag: impl Into<String>) -> Self {
		Self {
			handle: Some(handle),
			tag: tag.into(),
		}
	}

	pub fn tag(&self) -> &str {
		&self.tag
	}

	pub fn is_finished(&self) -> bool {
		self.handle.as_ref().unwrap().is_finished()
	}

	pub async fn wait(mut self) -> anyhow::Result<()> {
		self.handle
			.take()
			.expect("task already waited for")
			.await
			.with_context(|| format!("{}: task panicked", self.tag))?
			.with_context(|| format!("{}: task failed", self.tag))
	}
}

impl Drop for Task {
	fn drop(&mut self) {
		if let Some(handle) = self.handle.take() {
			handle.abort();
		}
	}
}

async fn retry_task<F: Future<Output = anyhow::Result<()>> + Send>(
	func: impl Fn() -> F,
	count: usize,
) -> anyhow::Result<()> {
	let mut count = count;
	loop {
		let Err(err) = func().await else {
			return Ok(());
		};
		if count == 0 {
			return Err(err);
		}

		count -= 1;
		tracing::warn!("failed to complete task: {:#}", err);
		tokio::time::sleep(std::time::Duration::from_secs(1)).await;
	}
}
