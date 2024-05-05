use std::sync::Arc;

use crate::global::Global;

pub async fn start(global: Arc<Global>) -> anyhow::Result<()> {
    std::future::pending::<()>().await;
	Ok(())
}
