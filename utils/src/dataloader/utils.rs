use tokio::sync::mpsc;

use super::types::DataLoaderInnerHolder;
use super::Loader;

pub(super) fn new_auto_loader<L: Loader<S>, S: Send + Sync + 'static>(
	mut auto_loader_rx: mpsc::Receiver<()>,
	duration: std::time::Duration,
	inner: DataLoaderInnerHolder<L, S>,
) -> tokio::task::AbortHandle {
	tokio::spawn(async move {
		while let Some(()) = auto_loader_rx.recv().await {
			let Some((batch_id, start)) = inner.lock().await.active_batch.as_ref().map(|b| (b.id, b.start)) else {
				continue;
			};

			if start.elapsed() < duration {
				tokio::time::sleep(duration - start.elapsed()).await;
			}

			let mut inner = inner.lock().await;

			if inner.active_batch.as_ref().map(|b| b.id != batch_id).unwrap_or(true) {
				continue;
			}

			tokio::spawn(inner.active_batch.take().unwrap().load(inner.semaphore.clone()));
		}
	})
	.abort_handle()
}
