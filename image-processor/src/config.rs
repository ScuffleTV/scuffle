use std::collections::HashMap;
use std::net::SocketAddr;

use scuffle_foundations::bootstrap::RuntimeSettings;
use scuffle_foundations::settings::auto_settings;
use scuffle_foundations::telemetry::settings::TelemetrySettings;
use url::Url;

#[auto_settings]
#[serde(default)]
pub struct ImageProcessorConfig {
	/// MongoDB database configuration
	pub database: DatabaseConfig,
	/// The drive configurations for the image processor
	pub drives: Vec<DriveConfig>,
	/// The event queues for the image processor
	pub event_queues: Vec<EventQueueConfig>,
	/// The worker configuration
	pub worker: WorkerConfig,
	/// The management configuration
	pub management: ManagementConfig,
	/// Telemetry configuration
	pub telemetry: TelemetrySettings,
	/// Runtime configuration
	pub runtime: RuntimeSettings,
}

#[auto_settings]
#[serde(default)]
pub struct ManagementConfig {
	/// The gRPC configuration
	pub grpc: GrpcConfig,
	/// The HTTP configuration
	pub http: HttpConfig,
}

#[auto_settings]
#[serde(default)]
pub struct GrpcConfig {
	/// Enable the gRPC server
	#[settings(default = true)]
	pub enabled: bool,
	/// The gRPC server address
	#[settings(default = SocketAddr::from(([0, 0, 0, 0], 50051)))]
	pub bind: SocketAddr,
}

#[auto_settings]
#[serde(default)]
pub struct HttpConfig {
	/// Enable the HTTP server
	#[settings(default = true)]
	pub enabled: bool,
	/// The HTTP server address
	#[settings(default = SocketAddr::from(([0, 0, 0, 0], 8080)))]
	pub bind: SocketAddr,
}

#[auto_settings]
#[serde(default)]
pub struct WorkerConfig {
	/// Enable the worker server
	#[settings(default = true)]
	pub enabled: bool,
	/// The number of workers to start
	/// Default is 0, which means the number of workers is equal to the number
	/// of CPU cores
	#[settings(default = 0)]
	pub concurrency: usize,
	/// The maximum number of errors before shutting down
	#[settings(default = 10)]
	pub error_threshold: usize,
	/// The delay before retrying after an error
	#[settings(default = std::time::Duration::from_secs(5))]
	#[serde(with = "humantime_serde")]
	pub error_delay: std::time::Duration,
	/// Polling interval for fetching jobs
	#[settings(default = std::time::Duration::from_secs(1))]
	#[serde(with = "humantime_serde")]
	pub polling_interval: std::time::Duration,
	/// Worker hold time, the time a worker holds a job. The job will be
	/// refreshed if the worker does not finish the job within this time. If the
	/// worker crashes or is killed, the job will be released after this time,
	/// at which point another worker can pick it up.
	#[settings(default = std::time::Duration::from_secs(60))]
	#[serde(with = "humantime_serde")]
	pub hold_time: std::time::Duration,
	/// Refresh interval for refreshing the job hold time
	/// Default is 1/3 of the hold time
	/// The refresh interval should be less than the hold time
	#[settings(default = std::time::Duration::from_secs(20))]
	#[serde(with = "humantime_serde")]
	pub refresh_interval: std::time::Duration,
}

#[auto_settings]
#[serde(default)]
pub struct DatabaseConfig {
	#[settings(default = "mongodb://localhost:27017/scuffle-image-processor".into())]
	pub uri: String,
}

#[auto_settings(impl_default = false)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum DriveConfig {
	/// Local drive
	Local(LocalDriveConfig),
	/// S3 bucket
	S3(S3DriveConfig),
	/// Memory drive
	Memory(MemoryDriveConfig),
	/// HTTP drive
	Http(HttpDriveConfig),
	/// Public web http drive
	PublicHttp(PublicHttpDriveConfig),
}

#[auto_settings]
pub struct LocalDriveConfig {
	/// The name of the drive
	pub name: String,
	/// The path to the local drive
	pub path: std::path::PathBuf,
	/// The drive mode
	#[serde(default)]
	pub mode: DriveMode,
}

#[auto_settings]
pub struct S3DriveConfig {
	/// The name of the drive
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
	pub prefix_path: Option<String>,
	/// Use path style
	#[serde(default)]
	pub force_path_style: Option<bool>,
	/// The drive mode
	#[serde(default)]
	pub mode: DriveMode,
	/// The maximum number of concurrent connections
	#[serde(default)]
	pub max_connections: Option<usize>,
	/// Default ACL for files
	#[serde(default)]
	pub acl: Option<String>,
}

fn default_region() -> String {
	"us-east-1".into()
}

#[auto_settings]
pub struct MemoryDriveConfig {
	/// The name of the drive
	pub name: String,
	/// The maximum capacity of the memory drive
	#[serde(default)]
	pub capacity: Option<usize>,
	/// The drive mode
	#[serde(default)]
	pub mode: DriveMode,
	/// Default ACL for files
	#[serde(default)]
	pub acl: Option<String>,
}

#[auto_settings(impl_default = false)]
pub struct HttpDriveConfig {
	/// The name of the drive
	pub name: String,
	/// The base URL for the HTTP drive
	pub url: Url,
	/// The timeout for the HTTP drive
	#[serde(default = "default_timeout")]
	#[serde(with = "humantime_serde")]
	pub timeout: Option<std::time::Duration>,
	/// Allow insecure TLS
	#[serde(default)]
	pub allow_insecure: bool,
	/// The drive mode
	#[serde(default)]
	pub mode: DriveMode,
	/// The maximum number of concurrent connections
	#[serde(default)]
	pub max_connections: Option<usize>,
	/// Additional headers for the HTTP drive
	#[serde(default)]
	pub headers: HashMap<String, String>,
	/// Default ACL for files
	#[serde(default)]
	pub acl: Option<String>,
}

fn default_timeout() -> Option<std::time::Duration> {
	Some(std::time::Duration::from_secs(30))
}

#[auto_settings]
#[serde(rename_all = "kebab-case")]
#[derive(Copy, PartialEq, Eq, Hash)]
pub enum DriveMode {
	/// Read only
	Read,
	#[settings(default)]
	/// Read and write
	ReadWrite,
	/// Write only
	Write,
}

/// Public http drives do not have a name because they will be invoked if the
/// input path is a URL that starts with 'http' or 'https'. Public http drives
/// can only be read-only. If you do not have a public http drive, the image
/// processor will not be able to download images using HTTP.
#[auto_settings]
pub struct PublicHttpDriveConfig {
	/// The timeout for the HTTP drive
	#[serde(default = "default_timeout")]
	#[serde(with = "humantime_serde")]
	pub timeout: Option<std::time::Duration>,
	/// Allow insecure TLS
	#[serde(default)]
	pub allow_insecure: bool,
	/// The maximum number of concurrent connections
	#[serde(default)]
	pub max_connections: Option<usize>,
	/// Additional headers for the HTTP drive
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
	/// For example: localhost:4222
	pub servers: Vec<String>,
	#[serde(default)]
	pub username: Option<String>,
	#[serde(default)]
	pub password: Option<String>,
	/// The message encoding for the event queue
	#[serde(default)]
	pub message_encoding: MessageEncoding,
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
	#[serde(with = "humantime_serde")]
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
	/// The message encoding for the event queue
	#[serde(default)]
	pub message_encoding: MessageEncoding,
}

#[auto_settings(impl_default = false)]
pub struct RedisEventQueueConfig {
	/// The name of the event queue
	pub name: String,
	/// The Redis URL, for example: redis://localhost:6379
	pub url: String,
	/// The message encoding for the event queue
	#[serde(default)]
	pub message_encoding: MessageEncoding,
}

#[auto_settings]
#[derive(Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum MessageEncoding {
	/// JSON encoding
	#[settings(default)]
	Json,
	/// Protobuf encoding
	Protobuf,
}
