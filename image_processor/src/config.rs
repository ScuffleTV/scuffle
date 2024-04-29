use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct ImageProcessorConfig {
	/// MongoDB database configuration
	pub database: DatabaseConfig,
	/// The disk configurations for the image processor
	pub disks: Vec<DiskConfig>,
	/// The event queues for the image processor
	pub event_queues: Vec<EventQueue>,
	/// Concurrency limit, defaults to number of CPUs
	pub concurrency: usize,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct DatabaseConfig {
	pub uri: String,
}

impl Default for DatabaseConfig {
	fn default() -> Self {
		Self {
			uri: "mongodb://localhost:27017".to_string(),
		}
	}
}

impl Default for ImageProcessorConfig {
	fn default() -> Self {
		Self {
			database: DatabaseConfig::default(),
			disks: vec![],
			event_queues: vec![],
			concurrency: num_cpus::get(),
		}
	}
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(tag = "kind")]
pub enum DiskConfig {
	/// Local disk
	Local(LocalDiskConfig),
	/// S3 bucket
	S3(S3DiskConfig),
	/// Memory disk
	Memory(MemoryDiskConfig),
	/// HTTP disk
	Http(HttpDiskConfig),
	/// Public web http disk
	PublicHttp(PublicHttpDiskConfig),
}

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct LocalDiskConfig {
	/// The name of the disk
	pub name: String,
	/// The path to the local disk
	pub path: std::path::PathBuf,
	/// The disk mode
	pub mode: DiskMode,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct S3DiskConfig {
	/// The name of the disk
	pub name: String,
	/// The S3 bucket name
	pub bucket: String,
	/// The S3 region
	pub region: String,
	/// The S3 access key
	pub access_key: String,
	/// The S3 secret key
	pub secret_key: String,
	/// The S3 endpoint
	pub endpoint: Option<String>,
	/// The S3 bucket prefix path
	pub path: Option<String>,
	/// Use path style
	pub path_style: bool,
	/// The disk mode
	pub mode: DiskMode,
	/// The maximum number of concurrent connections
	pub max_connections: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct MemoryDiskConfig {
	/// The name of the disk
	pub name: String,
	/// The maximum capacity of the memory disk
	pub capacity: Option<usize>,
	/// Global, shared memory disk for all tasks otherwise each task gets its
	/// own memory disk
	pub global: bool,
	/// The disk mode
	pub mode: DiskMode,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct HttpDiskConfig {
	/// The name of the disk
	pub name: String,
	/// The base URL for the HTTP disk
	pub base_url: String,
	/// The timeout for the HTTP disk
	pub timeout: Option<std::time::Duration>,
	/// Allow insecure TLS
	pub allow_insecure: bool,
	/// The disk mode
	pub mode: DiskMode,
	/// The maximum number of concurrent connections
	pub max_connections: Option<usize>,
	/// Additional headers for the HTTP disk
	pub headers: HashMap<String, String>,
	/// Write Method
	pub write_method: String,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
pub enum DiskMode {
	/// Read only
	Read,
	#[default]
	/// Read and write
	ReadWrite,
	/// Write only
	Write,
}


#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(default)]
/// Public http disks do not have a name because they will be invoked if the input path is a URL
/// that starts with 'http' or 'https'. Public http disks can only be read-only.
/// If you do not have a public http disk, the image processor will not be able to download images using HTTP.
pub struct PublicHttpDiskConfig {
	/// The timeout for the HTTP disk
	pub timeout: Option<std::time::Duration>,
	/// Allow insecure TLS
	pub allow_insecure: bool,
	/// The maximum number of concurrent connections
	pub max_connections: Option<usize>,
	/// Additional headers for the HTTP disk
	pub headers: HashMap<String, String>,
	/// Whitelist of allowed domains or IPs can be subnets or CIDR ranges
	/// IPs are compared after resolving the domain name
	pub whitelist: Vec<String>,
	/// Blacklist of disallowed domains or IPs can be subnets or CIDR ranges
	/// IPs are compared after resolving the domain name
	pub blacklist: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub enum EventQueue {
	Nats(NatsEventQueue),
	Http(HttpEventQueue),
	Redis(RedisEventQueue),
}

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct NatsEventQueue {
	pub name: String,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct HttpEventQueue {
	pub name: String,
	pub url: String,
	pub timeout: Option<std::time::Duration>,
	pub allow_insecure: bool,
	pub method: String,
	pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct RedisEventQueue {
	pub name: String,
	pub url: String,
}
