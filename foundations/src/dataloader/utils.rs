use super::types::DataLoaderInner;
use super::{CancelOnDrop, Loader};
use crate::runtime;

pub(super) fn new_auto_loader<L: Loader>(inner: DataLoaderInner<L>) -> CancelOnDrop {
	CancelOnDrop(
		runtime::spawn(async move {
			loop {
				let notify = inner.notify.notified();
				let Some((batch_id, start)) = inner.load_active_batch().await else {
					notify.await;
					continue;
				};

				drop(notify);

				let duration = inner.duration();
				if start.elapsed() < duration {
					tokio::time::sleep_until(start + duration).await;
				}

				let mut active_batch = inner.active_batch.write().await;

				if active_batch.as_ref().map(|b| b.id != batch_id).unwrap_or(true) {
					continue;
				}

				runtime::spawn(active_batch.take().unwrap().load(inner.semaphore.clone()));
			}
		})
		.abort_handle(),
	)
}
