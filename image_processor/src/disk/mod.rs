use bytes::Bytes;

use self::http::{HttpDisk, HttpDiskError};
use self::local::{LocalDisk, LocalDiskError};
use self::memory::{MemoryDisk, MemoryDiskError};
use self::public_http::{PublicHttpDisk, PublicHttpDiskError};
use self::s3::{S3Disk, S3DiskError};
use crate::config::DiskConfig;

pub mod http;
pub mod local;
pub mod memory;
pub mod public_http;
pub mod s3;

#[derive(Debug, thiserror::Error)]
pub enum DiskError {
	#[error("http: {0}")]
	Http(#[from] HttpDiskError),
	#[error("local: {0}")]
	Local(#[from] LocalDiskError),
	#[error("s3: {0}")]
	S3(#[from] S3DiskError),
	#[error("memory: {0}")]
	Memory(#[from] MemoryDiskError),
	#[error("public http: {0}")]
	PublicHttp(#[from] PublicHttpDiskError),
	#[error("not found")]
	NotFound,
	#[error("read only")]
	ReadOnly,
	#[error("write only")]
	WriteOnly,
}

#[derive(Debug, Clone, Default)]
pub struct DiskWriteOptions {
	pub cache_control: Option<String>,
	pub content_type: Option<String>,
	pub acl: Option<String>,
	pub content_disposition: Option<String>,
}

pub trait Disk {
	/// Get the name of the disk
	fn name(&self) -> &str;

	/// Read data from a disk
	fn read(&self, path: &str) -> impl std::future::Future<Output = Result<Bytes, DiskError>> + Send;

	/// Write data to a disk
	fn write(
		&self,
		path: &str,
		data: Bytes,
		options: Option<DiskWriteOptions>,
	) -> impl std::future::Future<Output = Result<(), DiskError>> + Send;

	/// Delete data from a disk
	fn delete(&self, path: &str) -> impl std::future::Future<Output = Result<(), DiskError>> + Send;

	/// Can be scoped to a specific request
	fn scoped(&self) -> Option<Self>
	where
		Self: Sized,
	{
		None
	}
}

#[derive(Debug)]
pub enum AnyDisk {
	Local(LocalDisk),
	S3(S3Disk),
	Memory(MemoryDisk),
	Http(HttpDisk),
	PublicHttp(PublicHttpDisk),
}

impl Disk for AnyDisk {
	fn name(&self) -> &str {
		match self {
			AnyDisk::Local(disk) => disk.name(),
			AnyDisk::S3(disk) => disk.name(),
			AnyDisk::Memory(disk) => disk.name(),
			AnyDisk::Http(disk) => disk.name(),
			AnyDisk::PublicHttp(disk) => disk.name(),
		}
	}

	async fn read(&self, path: &str) -> Result<Bytes, DiskError> {
		match self {
			AnyDisk::Local(disk) => disk.read(path).await,
			AnyDisk::S3(disk) => disk.read(path).await,
			AnyDisk::Memory(disk) => disk.read(path).await,
			AnyDisk::Http(disk) => disk.read(path).await,
			AnyDisk::PublicHttp(disk) => disk.read(path).await,
		}
	}

	async fn write(&self, path: &str, data: Bytes, options: Option<DiskWriteOptions>) -> Result<(), DiskError> {
		match self {
			AnyDisk::Local(disk) => disk.write(path, data, options).await,
			AnyDisk::S3(disk) => disk.write(path, data, options).await,
			AnyDisk::Memory(disk) => disk.write(path, data, options).await,
			AnyDisk::Http(disk) => disk.write(path, data, options).await,
			AnyDisk::PublicHttp(disk) => disk.write(path, data, options).await,
		}
	}

	async fn delete(&self, path: &str) -> Result<(), DiskError> {
		match self {
			AnyDisk::Local(disk) => disk.delete(path).await,
			AnyDisk::S3(disk) => disk.delete(path).await,
			AnyDisk::Memory(disk) => disk.delete(path).await,
			AnyDisk::Http(disk) => disk.delete(path).await,
			AnyDisk::PublicHttp(disk) => disk.delete(path).await,
		}
	}
}

pub async fn build_disk(config: &DiskConfig) -> Result<AnyDisk, DiskError> {
	match config {
		DiskConfig::Local(local) => Ok(AnyDisk::Local(LocalDisk::new(local).await?)),
		DiskConfig::S3(s3) => Ok(AnyDisk::S3(S3Disk::new(s3).await?)),
		DiskConfig::Memory(memory) => Ok(AnyDisk::Memory(MemoryDisk::new(memory).await?)),
		DiskConfig::Http(http) => Ok(AnyDisk::Http(HttpDisk::new(http).await?)),
		DiskConfig::PublicHttp(public_http) => Ok(AnyDisk::PublicHttp(PublicHttpDisk::new(public_http).await?)),
	}
}
