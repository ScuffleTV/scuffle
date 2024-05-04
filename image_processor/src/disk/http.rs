use bytes::Bytes;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Method;
use url::Url;

use super::{Disk, DiskError, DiskWriteOptions};
use crate::config::{DiskMode, HttpDiskConfig};

#[derive(Debug)]
pub struct HttpDisk {
	name: String,
	base_url: Url,
	mode: DiskMode,
	semaphore: Option<tokio::sync::Semaphore>,
	client: reqwest::Client,
}

#[derive(Debug, thiserror::Error)]
pub enum HttpDiskError {
	#[error("invalid path")]
	InvalidPath(#[from] url::ParseError),
	#[error("reqwest: {0}")]
	Reqwest(#[from] reqwest::Error),
	#[error("invalid header name")]
	InvalidHeaderName(#[from] reqwest::header::InvalidHeaderName),
	#[error("invalid header value")]
	InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),
}

impl HttpDisk {
	#[tracing::instrument(skip(config), name = "HttpDisk::new", fields(name = %config.name), err)]
	pub async fn new(config: &HttpDiskConfig) -> Result<Self, HttpDiskError> {
		tracing::debug!("setting up http disk");
		Ok(Self {
			name: config.name.clone(),
			base_url: config.url.clone(),
			mode: config.mode,
			semaphore: config.max_connections.map(|max| tokio::sync::Semaphore::new(max)),
			client: {
				let mut builder = reqwest::Client::builder();

				if let Some(timeout) = config.timeout {
					builder = builder.timeout(timeout);
				}

				if config.allow_insecure {
					builder = builder.danger_accept_invalid_certs(true);
				}

				let mut headers = HeaderMap::new();

				for (key, value) in &config.headers {
					headers.insert(key.parse::<HeaderName>()?, value.parse::<HeaderValue>()?);
				}

				builder = builder.default_headers(headers);

				builder.build().map_err(HttpDiskError::Reqwest)?
			},
		})
	}
}

impl Disk for HttpDisk {
	fn name(&self) -> &str {
		&self.name
	}

	#[tracing::instrument(skip(self), name = "HttpDisk::read", fields(name = %self.name), err)]
	async fn read(&self, path: &str) -> Result<Bytes, DiskError> {
		tracing::debug!("reading file");

		if self.mode == DiskMode::Write {
			return Err(DiskError::ReadOnly);
		}

		let _permit = if let Some(semaphore) = &self.semaphore {
			Some(semaphore.acquire().await)
		} else {
			None
		};

		let url = self.base_url.join(path).map_err(HttpDiskError::InvalidPath)?;

		let response = self.client.get(url).send().await.map_err(HttpDiskError::Reqwest)?;

		let response = response.error_for_status().map_err(HttpDiskError::Reqwest)?;

		Ok(response.bytes().await.map_err(HttpDiskError::Reqwest)?)
	}

	#[tracing::instrument(skip(self, data), name = "HttpDisk::write", fields(name = %self.name, size = data.len()), err)]
	async fn write(&self, path: &str, data: Bytes, options: Option<DiskWriteOptions>) -> Result<(), DiskError> {
		tracing::debug!("writing file");

		if self.mode == DiskMode::Read {
			return Err(DiskError::WriteOnly);
		}

		let _permit = if let Some(semaphore) = &self.semaphore {
			Some(semaphore.acquire().await)
		} else {
			None
		};

		let url = self.base_url.join(path).map_err(HttpDiskError::InvalidPath)?;

		let mut request = self
			.client
			.request(Method::POST, url)
			.body(data)
			.build()
			.map_err(HttpDiskError::Reqwest)?;

		if let Some(options) = options {
			if let Some(cache_control) = &options.cache_control {
				request.headers_mut().insert(
					reqwest::header::CACHE_CONTROL,
					cache_control.parse().map_err(HttpDiskError::InvalidHeaderValue)?,
				);
			}

			if let Some(content_type) = &options.content_type {
				request.headers_mut().insert(
					reqwest::header::CONTENT_TYPE,
					content_type.parse().map_err(HttpDiskError::InvalidHeaderValue)?,
				);
			}

			if let Some(acl) = &options.acl {
				request.headers_mut().insert(
					reqwest::header::HeaderName::from_static("x-amz-acl"),
					acl.parse().map_err(HttpDiskError::InvalidHeaderValue)?,
				);
			}
		}

		let resp = self.client.execute(request).await.map_err(HttpDiskError::Reqwest)?;

		resp.error_for_status().map_err(HttpDiskError::Reqwest)?;

		Ok(())
	}

	#[tracing::instrument(skip(self), name = "HttpDisk::delete", fields(name = %self.name), err)]
	async fn delete(&self, path: &str) -> Result<(), DiskError> {
		tracing::debug!("deleting file");

		if self.mode == DiskMode::Read {
			return Err(DiskError::WriteOnly);
		}

		let _permit = if let Some(semaphore) = &self.semaphore {
			Some(semaphore.acquire().await)
		} else {
			None
		};

		let url = self.base_url.join(path).map_err(HttpDiskError::InvalidPath)?;

		let response = self.client.delete(url).send().await.map_err(HttpDiskError::Reqwest)?;

		response.error_for_status().map_err(HttpDiskError::Reqwest)?;

		Ok(())
	}
}
