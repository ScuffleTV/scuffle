use common::config::{S3BucketConfig, S3CredentialsConfig};
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

	/// Allow http downloads
	pub allow_http: bool,
}

impl Default for ImageProcessorConfig {
	fn default() -> Self {
		Self {
			source_bucket: S3BucketConfig {
				name: "scuffle-image-processor".to_owned(),
				endpoint: Some("http://localhost:9000".to_owned()),
				region: "us-east-1".to_owned(),
				credentials: S3CredentialsConfig {
					access_key: Some("root".to_owned()),
					secret_key: Some("scuffle123".to_owned()),
				},
			},
			target_bucket: S3BucketConfig {
				name: "scuffle-image-processor-public".to_owned(),
				endpoint: Some("http://localhost:9000".to_owned()),
				region: "us-east-1".to_owned(),
				credentials: S3CredentialsConfig {
					access_key: Some("root".to_owned()),
					secret_key: Some("scuffle123".to_owned()),
				},
			},
			concurrency: num_cpus::get(),
			instance_id: Ulid::new(),
			allow_http: true,
		}
	}
}
