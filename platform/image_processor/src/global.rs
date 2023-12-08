use crate::config::ImageProcessorConfig;

pub trait ImageProcessorState {
	fn s3_source_bucket(&self) -> &s3::Bucket;
	fn s3_target_bucket(&self) -> &s3::Bucket;
}

pub trait ImageProcessorGlobal:
	common::global::GlobalCtx
	+ common::global::GlobalConfigProvider<ImageProcessorConfig>
	+ common::global::GlobalNats
	+ common::global::GlobalDb
	+ common::global::GlobalConfig
	+ ImageProcessorState
	+ Send
	+ Sync
	+ 'static
{
}

impl<T> ImageProcessorGlobal for T where
	T: common::global::GlobalCtx
		+ common::global::GlobalConfigProvider<ImageProcessorConfig>
		+ common::global::GlobalNats
		+ common::global::GlobalDb
		+ common::global::GlobalConfig
		+ ImageProcessorState
		+ Send
		+ Sync
		+ 'static
{
}
