use std::task::Poll;

use aws_config::Region;
use aws_credential_types::Credentials;
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::delete_object::DeleteObjectError;
use aws_sdk_s3::operation::get_object::{GetObjectError, GetObjectOutput};
use aws_sdk_s3::operation::put_object::PutObjectError;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::ObjectCannedAcl;
use bytes::Bytes;

#[cfg(feature = "config")]
use crate::config::S3BucketConfig;
use crate::config::S3CredentialsConfig;

#[derive(Debug, Clone)]
pub struct Bucket {
	name: String,
	client: aws_sdk_s3::Client,
}

#[derive(Debug, Clone, Default)]
pub struct PutObjectOptions {
	pub acl: Option<ObjectCannedAcl>,
	pub content_type: Option<String>,
}

#[cfg(feature = "config")]
impl From<S3CredentialsConfig> for Credentials {
	fn from(value: S3CredentialsConfig) -> Self {
		Self::from_keys(
			value.access_key.unwrap_or_default(),
			value.secret_key.unwrap_or_default(),
			None,
		)
	}
}

#[cfg(feature = "config")]
impl S3BucketConfig {
	pub fn setup(&self) -> Bucket {
		Bucket::new(
			self.name.clone(),
			self.credentials.clone().into(),
			Region::new(self.region.clone()),
			self.endpoint.clone(),
		)
	}
}

#[pin_project::pin_project]
pub struct AsyncStreamBody<T>(#[pin] pub T);

impl<T> http_body::Body for AsyncStreamBody<T>
where
	T: futures_util::stream::TryStream + Send + Sync + 'static,
	T::Ok: Into<Bytes> + Send + Sync + 'static,
	T::Error: Into<aws_smithy_types::body::Error> + Send + Sync + 'static,
{
	type Data = Bytes;
	type Error = aws_smithy_types::body::Error;

	fn poll_frame(
		self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
		let this = self.project();

		match this.0.try_poll_next(cx) {
			Poll::Pending => Poll::Pending,
			Poll::Ready(Some(Ok(bytes))) => Poll::Ready(Some(Ok(http_body::Frame::data(bytes.into())))),
			Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e.into()))),
			Poll::Ready(None) => Poll::Ready(None),
		}
	}
}

impl<T> From<AsyncStreamBody<T>> for ByteStream
where
	T: futures_util::stream::TryStream + Send + Sync + 'static,
	T::Ok: Into<Bytes> + Send + Sync + 'static,
	T::Error: Into<aws_smithy_types::body::Error> + Send + Sync + 'static,
{
	fn from(value: AsyncStreamBody<T>) -> Self {
		ByteStream::from_body_1_x(value)
	}
}

impl Bucket {
	pub fn new(name: String, credentials: Credentials, region: Region, endpoint: Option<String>) -> Self {
		let config = if let Some(endpoint) = endpoint {
			aws_sdk_s3::config::Builder::new().endpoint_url(endpoint)
		} else {
			aws_sdk_s3::config::Builder::new()
		}
		.region(region)
		.credentials_provider(credentials)
		.force_path_style(true)
		.build();

		let client = aws_sdk_s3::Client::from_conf(config);

		Self { name, client }
	}

	pub fn name(&self) -> &str {
		&self.name
	}

	pub async fn get_object(&self, key: &str) -> Result<GetObjectOutput, SdkError<GetObjectError>> {
		let resp = self.client.get_object().bucket(self.name()).key(key).send().await?;

		Ok(resp)
	}

	pub async fn put_object(
		&self,
		key: impl Into<String>,
		body: impl Into<ByteStream>,
		options: Option<PutObjectOptions>,
	) -> Result<(), SdkError<PutObjectError>> {
		let options = options.unwrap_or_default();

		self.client
			.put_object()
			.bucket(self.name())
			.key(key)
			.body(body.into())
			.set_acl(options.acl)
			.set_content_type(options.content_type)
			.send()
			.await?;

		Ok(())
	}

	pub async fn delete_object(&self, key: &str) -> Result<(), SdkError<DeleteObjectError>> {
		self.client.delete_object().bucket(self.name()).key(key).send().await?;

		Ok(())
	}
}
