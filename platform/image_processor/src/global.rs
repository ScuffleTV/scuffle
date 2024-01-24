use binary_helper::s3::Bucket;

use crate::config::ImageProcessorConfig;

pub trait ImageProcessorState {
	fn s3_source_bucket(&self) -> &Bucket;
	fn s3_target_bucket(&self) -> &Bucket;
	fn http_client(&self) -> &reqwest::Client;
}

pub trait ImageProcessorGlobal:
	binary_helper::global::GlobalCtx
	+ binary_helper::global::GlobalConfigProvider<ImageProcessorConfig>
	+ binary_helper::global::GlobalNats
	+ binary_helper::global::GlobalDb
	+ binary_helper::global::GlobalConfig
	+ ImageProcessorState
	+ Send
	+ Sync
	+ 'static
{
}

impl<T> ImageProcessorGlobal for T where
	T: binary_helper::global::GlobalCtx
		+ binary_helper::global::GlobalConfigProvider<ImageProcessorConfig>
		+ binary_helper::global::GlobalNats
		+ binary_helper::global::GlobalDb
		+ binary_helper::global::GlobalConfig
		+ ImageProcessorState
		+ Send
		+ Sync
		+ 'static
{
}
