use bytes::Bytes;
use http::{HeaderName, HeaderValue};

use super::{Disk, DiskError, DiskWriteOptions};
use crate::config::PublicHttpDiskConfig;

pub const PUBLIC_HTTP_DISK_NAME: &str = "__public_http";

#[derive(Debug)]
pub struct PublicHttpDisk {
	client: reqwest::Client,
	semaphore: Option<tokio::sync::Semaphore>,
}

#[derive(Debug, thiserror::Error)]
pub enum PublicHttpDiskError {
	#[error("reqwest: {0}")]
	Reqwest(#[from] reqwest::Error),
	#[error("invalid header name")]
	InvalidHeaderName(#[from] reqwest::header::InvalidHeaderName),
	#[error("invalid header value")]
	InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),
	#[error("unsupported: {0}")]
	Unsupported(&'static str),
}

impl PublicHttpDisk {
	#[tracing::instrument(skip(config), name = "PublicHttpDisk::new", err)]
	pub async fn new(config: &PublicHttpDiskConfig) -> Result<Self, PublicHttpDiskError> {
		tracing::debug!("setting up public http disk");
		if !config.blacklist.is_empty() || !config.whitelist.is_empty() {
			tracing::error!("blacklist and whitelist are not supported for public http disk");
			return Err(PublicHttpDiskError::Unsupported("blacklist and whitelist"));
		}

		Ok(Self {
			client: {
				let mut builder = reqwest::Client::builder();

				if let Some(timeout) = config.timeout {
					builder = builder.timeout(timeout);
				}

				if config.allow_insecure {
					builder = builder.danger_accept_invalid_certs(true);
				}

				let mut headers = reqwest::header::HeaderMap::new();

				for (key, value) in &config.headers {
					headers.insert(key.parse::<HeaderName>()?, value.parse::<HeaderValue>()?);
				}

				builder = builder.default_headers(headers);

				builder.build().map_err(|e| PublicHttpDiskError::Reqwest(e))?
			},
			semaphore: config.max_connections.map(|max| tokio::sync::Semaphore::new(max)),
		})
	}
}

impl Disk for PublicHttpDisk {
	fn name(&self) -> &str {
		PUBLIC_HTTP_DISK_NAME
	}

	#[tracing::instrument(skip(self), name = "PublicHttpDisk::read", err)]
	async fn read(&self, path: &str) -> Result<Bytes, DiskError> {
		tracing::debug!("reading file");

		let _permit = if let Some(semaphore) = &self.semaphore {
			Some(semaphore.acquire().await)
		} else {
			None
		};

		let response = self.client.get(path).send().await.map_err(PublicHttpDiskError::Reqwest)?;

		let response = response.error_for_status().map_err(PublicHttpDiskError::Reqwest)?;

		Ok(response.bytes().await.map_err(PublicHttpDiskError::Reqwest)?)
	}

	#[tracing::instrument(skip(self, data), name = "PublicHttpDisk::write", fields(size = data.len()), err)]
	async fn write(&self, path: &str, data: Bytes, options: Option<DiskWriteOptions>) -> Result<(), DiskError> {
		tracing::error!("writing is not supported for public http disk");
		Err(DiskError::ReadOnly)
	}

	#[tracing::instrument(skip(self), name = "PublicHttpDisk::delete", err)]
	async fn delete(&self, path: &str) -> Result<(), DiskError> {
		tracing::error!("deleting is not supported for public http disk");
		Err(DiskError::ReadOnly)
	}
}
