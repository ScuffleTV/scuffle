use common::config::S3BucketConfig;
use ulid::Ulid;

#[derive(Debug, Clone, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
pub struct ImageProcessorConfig {
	/// The S3 Bucket which contains the source images
	pub source_bucket: S3BucketConfig,

	/// The S3 Bucket which will contain the target images
	pub target_bucket: S3BucketConfig,

	/// Concurrency limit, defaults to number of CPUs
	pub concurrency: usize,

	/// Instance ID (defaults to a random ULID)
	pub instance_id: Ulid,
}

impl Default for ImageProcessorConfig {
	fn default() -> Self {
		Self {
			source_bucket: S3BucketConfig::default(),
			target_bucket: S3BucketConfig::default(),
			concurrency: num_cpus::get(),
			instance_id: Ulid::new(),
		}
	}
}
