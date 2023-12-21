use std::path::PathBuf;
// use std::sync::Arc;

// use common::context::Handler;

// use super::global::GlobalState;

// pub async fn teardown(global: Arc<GlobalState>, handler: Handler) {
// 	drop(global);
// 	handler.cancel().await;
// }

pub fn asset_path(name: &str) -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.unwrap()
		.join("assets")
		.join(name)
}
