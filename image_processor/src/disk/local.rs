use std::path::PathBuf;

use bytes::Bytes;

use super::{Disk, DiskError, DiskWriteOptions};
use crate::config::{DiskMode, LocalDiskConfig};

#[derive(Debug)]
pub struct LocalDisk {
	name: String,
	mode: DiskMode,
	path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum LocalDiskError {
	#[error("io: {0}")]
	Io(#[from] std::io::Error),
}

impl LocalDisk {
	#[tracing::instrument(skip(config), name = "LocalDisk::new", fields(name = %config.name), err)]
	pub async fn new(config: &LocalDiskConfig) -> Result<Self, LocalDiskError> {
		tracing::debug!("setting up local disk");

		if !config.path.exists() {
			tokio::fs::create_dir_all(&config.path).await.map_err(LocalDiskError::Io)?;
		}

		Ok(Self {
			name: config.name.clone(),
			mode: config.mode,
			path: config.path.clone(),
		})
	}
}

impl Disk for LocalDisk {
	fn name(&self) -> &str {
		&self.name
	}

	#[tracing::instrument(skip(self), name = "LocalDisk::read", err)]
	async fn read(&self, path: &str) -> Result<Bytes, DiskError> {
		tracing::debug!("reading file");

		if self.mode == DiskMode::Write {
			return Err(DiskError::ReadOnly);
		}

		let path = self.path.join(path);
		Ok(tokio::fs::read(path).await.map_err(LocalDiskError::Io)?.into())
	}

	#[tracing::instrument(skip(self, data), name = "LocalDisk::write", err, fields(size = data.len()))]
	async fn write(&self, path: &str, data: Bytes, options: Option<DiskWriteOptions>) -> Result<(), DiskError> {
		tracing::debug!("writing file");

		if self.mode == DiskMode::Read {
			return Err(DiskError::WriteOnly);
		}

		let path = self.path.join(path);
		Ok(tokio::fs::write(path, data).await.map_err(LocalDiskError::Io)?)
	}

	#[tracing::instrument(skip(self), name = "LocalDisk::delete", err)]
	async fn delete(&self, path: &str) -> Result<(), DiskError> {
		tracing::debug!("deleting file");

		if self.mode == DiskMode::Read {
			return Err(DiskError::WriteOnly);
		}

		let path = self.path.join(path);
		tokio::fs::remove_file(path).await.map_err(LocalDiskError::Io)?;
		Ok(())
	}
}
