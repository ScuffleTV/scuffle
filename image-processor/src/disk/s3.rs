use aws_config::{AppName, Region, SdkConfig};
use aws_sdk_s3::config::{Credentials, SharedCredentialsProvider};
use aws_sdk_s3::operation::delete_object::DeleteObjectError;
use aws_sdk_s3::operation::get_object::GetObjectError;
use aws_sdk_s3::operation::put_object::PutObjectError;
use aws_smithy_runtime_api::client::orchestrator::HttpResponse;
use aws_smithy_runtime_api::client::result::SdkError;
use bytes::Bytes;
use scuffle_foundations::service_info;

use super::{Drive, DriveError, DriveWriteOptions};
use crate::config::{DriveMode, S3DriveConfig};

#[derive(Debug)]
pub struct S3Drive {
	name: String,
	mode: DriveMode,
	client: aws_sdk_s3::Client,
	bucket: String,
}

#[derive(Debug, thiserror::Error)]
pub enum S3DriveError {
	#[error("s3: {0}")]
	S3Error(#[from] aws_sdk_s3::Error),
	#[error("byte stream: {0}")]
	ByteStreamError(#[from] aws_smithy_types::byte_stream::error::Error),
	#[error("read: {0}")]
	ReadError(#[from] SdkError<GetObjectError, HttpResponse>),
	#[error("write: {0}")]
	WriteError(#[from] SdkError<PutObjectError, HttpResponse>),
	#[error("delete: {0}")]
	DeleteError(#[from] SdkError<DeleteObjectError, HttpResponse>),
}

impl S3Drive {
	#[tracing::instrument(skip(config), name = "S3Disk::new", fields(name = %config.name), err)]
	pub async fn new(config: &S3DriveConfig) -> Result<Self, DriveError> {
		tracing::debug!("setting up s3 disk");
		Ok(Self {
			name: config.name.clone(),
			mode: config.mode,
			client: aws_sdk_s3::Client::new(&{
				let mut builder = SdkConfig::builder();

				builder.set_app_name(Some(AppName::new(service_info!().name).unwrap()));

				builder.set_region(Some(Region::new(config.region.clone())));

				builder.set_credentials_provider(Some(SharedCredentialsProvider::new(Credentials::new(
					config.access_key.clone(),
					config.secret_key.clone(),
					None,
					None,
					"ConfiguredCredentialsProvider",
				))));

				builder.build()
			}),
			bucket: config.bucket.clone(),
		})
	}
}

impl Drive for S3Drive {
	fn name(&self) -> &str {
		&self.name
	}

	#[tracing::instrument(skip(self), name = "S3Disk::read", err)]
	async fn read(&self, path: &str) -> Result<Bytes, DriveError> {
		if self.mode == DriveMode::Write {
			return Err(DriveError::ReadOnly);
		}

		let result = self
			.client
			.get_object()
			.bucket(&self.bucket)
			.key(path)
			.send()
			.await
			.map_err(S3DriveError::from)?;

		let bytes = result.body.collect().await.map_err(S3DriveError::from)?;

		Ok(bytes.into_bytes())
	}

	#[tracing::instrument(skip(self, data), name = "S3Disk::write", err, fields(size = data.len()))]
	async fn write(&self, path: &str, data: Bytes, options: Option<DriveWriteOptions>) -> Result<(), DriveError> {
		if self.mode == DriveMode::Read {
			return Err(DriveError::WriteOnly);
		}

		let mut req = self.client.put_object().bucket(&self.bucket).key(path).body(data.into());

		if let Some(options) = options {
			if let Some(cache_control) = &options.cache_control {
				req = req.cache_control(cache_control);
			}
			if let Some(content_type) = &options.content_type {
				req = req.content_type(content_type);
			}
			if let Some(content_disposition) = &options.content_disposition {
				req = req.content_disposition(content_disposition);
			}
			if let Some(acl) = &options.acl {
				req = req.acl(acl.as_str().into());
			}
		}

		req.send().await.map_err(S3DriveError::from)?;

		Ok(())
	}

	#[tracing::instrument(skip(self), name = "S3Disk::delete", err)]
	async fn delete(&self, path: &str) -> Result<(), DriveError> {
		if self.mode == DriveMode::Read {
			return Err(DriveError::WriteOnly);
		}

		self.client
			.delete_object()
			.bucket(&self.bucket)
			.key(path)
			.send()
			.await
			.map_err(S3DriveError::from)?;

		Ok(())
	}
}
