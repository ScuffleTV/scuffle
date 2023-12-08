use anyhow::Context;
use s3::Region;
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

	/// Working directory (defaults to the /tmp/{instance_id} directory)
	pub working_directory: Option<String>,
}

impl Default for ImageProcessorConfig {
	fn default() -> Self {
		Self {
			source_bucket: S3BucketConfig::default(),
			target_bucket: S3BucketConfig::default(),
			concurrency: num_cpus::get(),
			instance_id: Ulid::new(),
			working_directory: None,
		}
	}
}

#[derive(Debug, Default, Clone, PartialEq, config::Config, serde::Deserialize)]
pub struct S3CredentialsConfig {
	/// The access key for the S3 bucket
	pub access_key: Option<String>,

	/// The secret key for the S3 bucket
	pub secret_key: Option<String>,
}

impl From<S3CredentialsConfig> for s3::creds::Credentials {
	fn from(value: S3CredentialsConfig) -> Self {
		Self {
			access_key: value.access_key,
			secret_key: value.secret_key,
			security_token: None,
			session_token: None,
			expiration: None,
		}
	}
}

#[derive(Debug, Clone, PartialEq, config::Config, serde::Deserialize)]
#[serde(default)]
pub struct S3BucketConfig {
	/// The name of the S3 bucket
	pub name: String,

	/// The region the S3 bucket is in
	pub region: String,

	/// The custom endpoint for the S3 bucket
	pub endpoint: Option<String>,

	/// The credentials for the S3 bucket
	pub credentials: S3CredentialsConfig,
}

impl Default for S3BucketConfig {
	fn default() -> Self {
		Self {
			name: "scuffle-image-processor".to_owned(),
			region: Region::UsEast1.to_string(),
			endpoint: Some("http://localhost:9000".to_string()),
			credentials: S3CredentialsConfig::default(),
		}
	}
}

impl S3BucketConfig {
	pub async fn setup(&self) -> anyhow::Result<s3::Bucket> {
		let region: s3::Region = self.region.parse()?;
		Ok(s3::Bucket::new(&self.name, region, self.credentials.clone().into())
			.context("failed to create S3 bucket")?
			.with_path_style())
	}
}
