use std::collections::HashMap;

use bytes::Bytes;
use tokio::sync::RwLock;

use super::{Disk, DiskError, DiskWriteOptions};
use crate::config::{DiskMode, MemoryDiskConfig};

#[derive(Debug)]
struct FileHolder {
	remaining_capacity: usize,
	files: HashMap<String, MemoryFile>,
}

impl FileHolder {
	fn get(&self, path: &str) -> Option<&MemoryFile> {
		self.files.get(path)
	}

	fn insert(&mut self, path: String, file: MemoryFile) -> Result<Option<MemoryFile>, MemoryDiskError> {
		if file.data.len() > self.remaining_capacity {
			return Err(MemoryDiskError::NoSpaceLeft);
		}

		self.remaining_capacity -= file.data.len();
		self.files
			.insert(path, file)
			.map(|file| {
				self.remaining_capacity += file.data.len();
				Ok(file)
			})
			.transpose()
	}

	fn remove(&mut self, path: &str) -> Option<MemoryFile> {
		let file = self.files.remove(path)?;
		self.remaining_capacity += file.data.len();
		Some(file)
	}
}

#[derive(Debug)]
pub struct MemoryDisk {
	name: String,
	mode: DiskMode,
	files: RwLock<FileHolder>,
	global: bool,
}

#[derive(Debug, Clone)]
pub struct MemoryFile {
	data: Bytes,
	_options: DiskWriteOptions,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum MemoryDiskError {
	#[error("no space left on disk")]
	NoSpaceLeft,
}

impl MemoryDisk {
	#[tracing::instrument(skip(config), name = "MemoryDisk::new", fields(name = %config.name), err)]
	pub async fn new(config: &MemoryDiskConfig) -> Result<Self, MemoryDiskError> {
		tracing::debug!("setting up memory disk");
		Ok(Self {
			name: config.name.clone(),
			mode: config.mode,
			global: config.global,
			files: RwLock::new(FileHolder {
				remaining_capacity: config.capacity.unwrap_or(usize::MAX),
				files: HashMap::new(),
			}),
		})
	}
}

impl Disk for MemoryDisk {
	fn name(&self) -> &str {
		&self.name
	}

	#[tracing::instrument(skip(self), name = "MemoryDisk::read", err)]
	async fn read(&self, path: &str) -> Result<Bytes, DiskError> {
		tracing::debug!("reading file");

		if self.mode == DiskMode::Write {
			return Err(DiskError::ReadOnly);
		}

		Ok(self
			.files
			.read()
			.await
			.get(path)
			.map(|file| file.data.clone())
			.ok_or(DiskError::NotFound)?)
	}

	#[tracing::instrument(skip(self, data), name = "MemoryDisk::write", err, fields(size = data.len()))]
	async fn write(&self, path: &str, data: Bytes, options: Option<DiskWriteOptions>) -> Result<(), DiskError> {
		tracing::debug!("writing file");

		if self.mode == DiskMode::Read {
			return Err(DiskError::WriteOnly);
		}

		let mut files = self.files.write().await;

		files.insert(
			path.to_owned(),
			MemoryFile {
				data,
				_options: options.unwrap_or_default(),
			},
		)?;

		Ok(())
	}

	#[tracing::instrument(skip(self), name = "MemoryDisk::delete", err)]
	async fn delete(&self, path: &str) -> Result<(), DiskError> {
		tracing::debug!("deleting file");

		if self.mode == DiskMode::Read {
			return Err(DiskError::WriteOnly);
		}

		self.files.write().await.remove(path).ok_or(DiskError::NotFound)?;
		Ok(())
	}

	fn scoped(&self) -> Option<Self>
	where
		Self: Sized,
	{
		if self.global {
			return None;
		}

		Some(Self {
			name: self.name.clone(),
			mode: self.mode,
			global: false,
			files: RwLock::new(FileHolder {
				remaining_capacity: 0,
				files: HashMap::new(),
			}),
		})
	}
}
