use std::collections::HashMap;

use scuffle_foundations::{bootstrap::RuntimeSettings, settings::auto_settings, telemetry::settings::TelemetrySettings};
use url::Url;

#[auto_settings]
#[serde(default)]
pub struct ImageProcessorConfig {
	/// MongoDB database configuration
	pub database: DatabaseConfig,
	/// The disk configurations for the image processor
	pub disks: Vec<DiskConfig>,
	/// The event queues for the image processor
	pub event_queues: Vec<EventQueueConfig>,
	/// Concurrency limit, defaults to number of CPUs
	/// 0 means all CPUs
	#[settings(default = 0)]
	pub concurrency: usize,

	/// Telemetry configuration
	pub telemetry: TelemetrySettings,
	/// Runtime configuration
	pub runtime: RuntimeSettings,
}

#[auto_settings]
#[serde(default)]
pub struct DatabaseConfig {
	#[settings(default = "mongodb://localhost:27017".into())]
	pub uri: String,
}

#[auto_settings(impl_default = false)]
#[serde(tag = "kind", rename_all = "kebab-case")]
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

#[auto_settings]
pub struct LocalDiskConfig {
	/// The name of the disk
	pub name: String,
	/// The path to the local disk
	pub path: std::path::PathBuf,
	/// The disk mode
	#[serde(default)]
	pub mode: DiskMode,
}

#[auto_settings]
pub struct S3DiskConfig {
	/// The name of the disk
	pub name: String,
	/// The S3 bucket name
	pub bucket: String,
	/// The S3 access key
	pub access_key: String,
	/// The S3 secret key
	pub secret_key: String,
	/// The S3 region
	#[serde(default = "default_region")]
	pub region: String,
	/// The S3 endpoint
	#[serde(default)]
	pub endpoint: Option<String>,
	/// The S3 bucket prefix path
	#[serde(default)]
	pub path: Option<String>,
	/// Use path style
	#[serde(default)]
	pub path_style: bool,
	/// The disk mode
	#[serde(default)]
	pub mode: DiskMode,
	/// The maximum number of concurrent connections
	#[serde(default)]
	pub max_connections: Option<usize>,
}

fn default_region() -> String {
	"us-east-1".into()
}

#[auto_settings]
pub struct MemoryDiskConfig {
	/// The name of the disk
	pub name: String,
	/// The maximum capacity of the memory disk
	#[serde(default)]
	pub capacity: Option<usize>,
	/// Global, shared memory disk for all tasks otherwise each task gets its
	/// own memory disk
	#[serde(default = "default_true")]
	pub global: bool,
	/// The disk mode
	#[serde(default)]
	pub mode: DiskMode,
}

fn default_true() -> bool {
	true
}

#[auto_settings(impl_default = false)]
pub struct HttpDiskConfig {
	/// The name of the disk
	pub name: String,
	/// The base URL for the HTTP disk
	pub url: Url,
	/// The timeout for the HTTP disk
	#[serde(default = "default_timeout")]
	pub timeout: Option<std::time::Duration>,
	/// Allow insecure TLS
	#[serde(default)]
	pub allow_insecure: bool,
	/// The disk mode
	#[serde(default)]
	pub mode: DiskMode,
	/// The maximum number of concurrent connections
	#[serde(default)]
	pub max_connections: Option<usize>,
	/// Additional headers for the HTTP disk
	#[serde(default)]
	pub headers: HashMap<String, String>,
}

fn default_timeout() -> Option<std::time::Duration> {
	Some(std::time::Duration::from_secs(30))
}

#[auto_settings]
#[serde(rename_all = "kebab-case")]
#[derive(Copy, PartialEq, Eq, Hash)]
pub enum DiskMode {
	/// Read only
	Read,
	#[settings(default)]
	/// Read and write
	ReadWrite,
	/// Write only
	Write,
}

/// Public http disks do not have a name because they will be invoked if the
/// input path is a URL that starts with 'http' or 'https'. Public http disks
/// can only be read-only. If you do not have a public http disk, the image
/// processor will not be able to download images using HTTP.
#[auto_settings]
pub struct PublicHttpDiskConfig {
	/// The timeout for the HTTP disk
	#[serde(default = "default_timeout")]
	pub timeout: Option<std::time::Duration>,
	/// Allow insecure TLS
	#[serde(default)]
	pub allow_insecure: bool,
	/// The maximum number of concurrent connections
	#[serde(default)]
	pub max_connections: Option<usize>,
	/// Additional headers for the HTTP disk
	#[serde(default)]
	pub headers: HashMap<String, String>,
	/// Whitelist of allowed domains or IPs can be subnets or CIDR ranges
	/// IPs are compared after resolving the domain name
	#[serde(default)]
	pub whitelist: Vec<String>,
	/// Blacklist of disallowed domains or IPs can be subnets or CIDR ranges
	/// IPs are compared after resolving the domain name
	#[serde(default)]
	pub blacklist: Vec<String>,
}

#[auto_settings(impl_default = false)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum EventQueueConfig {
	Nats(NatsEventQueueConfig),
	Http(HttpEventQueueConfig),
	Redis(RedisEventQueueConfig),
}

#[auto_settings(impl_default = false)]
pub struct NatsEventQueueConfig {
	/// The name of the event queue
	pub name: String,
	/// The Nats URL
	/// For example: nats://localhost:4222
	pub url: String,
	/// Allow Protobuf messages
	#[serde(default)]
	pub allow_protobuf: bool,
}

#[auto_settings(impl_default = false)]
pub struct HttpEventQueueConfig {
	/// The name of the event queue
	pub name: String,
	/// The base URL for the HTTP event queue
	pub url: Url,
	/// The timeout for the HTTP event queue
	/// Default is 30 seconds
	#[serde(default = "default_timeout")]
	pub timeout: Option<std::time::Duration>,
	/// Allow insecure TLS (if the url is https, do not verify the certificate)
	#[serde(default)]
	pub allow_insecure: bool,
	/// Additional headers for the HTTP event queue
	/// Can be used to set the authorization header
	/// Default is empty
	#[serde(default)]
	pub headers: HashMap<String, String>,
	/// The maximum number of concurrent connections
	/// Default is None
	#[serde(default)]
	pub max_connections: Option<usize>,
	/// Allow Protobuf messages
	#[serde(default)]
	pub allow_protobuf: bool,
}

#[auto_settings(impl_default = false)]
pub struct RedisEventQueueConfig {
	/// The name of the event queue
	pub name: String,
	/// The Redis URL, for example: redis://localhost:6379
	pub url: String,
	/// Allow Protobuf messages
	#[serde(default)]
	pub allow_protobuf: bool,
}
