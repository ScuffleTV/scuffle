use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use hyper_util::rt::TokioExecutor;

use super::stream::MakeService;
use super::{Error, Server};

impl From<SocketAddr> for ServerBuilder {
	fn from(addr: SocketAddr) -> Self {
		Self::default().bind(addr)
	}
}

#[derive(Debug, Clone)]
pub struct ServerBuilder {
	/// Bind address for the server on insecure connections.
	bind: Option<std::net::SocketAddr>,
	/// Bind address for the server on secure connections.
	#[cfg(feature = "http-tls")]
	insecure_bind: Option<std::net::SocketAddr>,
	/// TLS configuration for secure connections.
	#[cfg(feature = "http-tls")]
	tls: Option<rustls::ServerConfig>,
	http1_2: hyper_util::server::conn::auto::Builder<TokioExecutor>,
	#[cfg(feature = "http3")]
	quic: Option<super::Quic>,
	keep_alive_timeout: Option<std::time::Duration>,
	worker_count: usize,
}

impl Default for ServerBuilder {
	fn default() -> Self {
		Self {
			bind: None,
			#[cfg(feature = "http-tls")]
			insecure_bind: None,
			#[cfg(feature = "http-tls")]
			tls: None,
			http1_2: hyper_util::server::conn::auto::Builder::new(TokioExecutor::new()),
			#[cfg(feature = "http3")]
			quic: None,
			keep_alive_timeout: Some(Duration::from_secs(30)),
			worker_count: 1,
		}
	}
}

impl ServerBuilder {
	/// Bind the server to an address.
	pub fn bind(mut self, addr: std::net::SocketAddr) -> Self {
		self.bind = Some(addr);
		self
	}

	/// Configure the server to use HTTP/1 and HTTP/2.
	pub fn with_http(mut self, http: hyper_util::server::conn::auto::Builder<TokioExecutor>) -> Self {
		self.http1_2 = http;
		self
	}

	#[cfg(feature = "http-tls")]
	pub fn with_tls(mut self, tls: rustls::ServerConfig) -> Self {
		self.tls = Some(tls);
		self
	}

	/// Configure the server to use HTTP/3.
	#[cfg(feature = "http3")]
	pub fn with_http3(mut self, h3: h3::server::Builder, config: quinn::ServerConfig) -> Self {
		self.quic = Some(super::Quic {
			h3: Arc::new(h3),
			config,
		});
		self
	}

	/// Bind the server to an insecure address.
	/// If `allow_connections` is `false`, the server will not accept any
	/// connections, and will do a http redirect to the secure address.
	#[cfg(feature = "http-tls")]
	pub fn with_insecure(mut self, addr: std::net::SocketAddr) -> Self {
		self.insecure_bind = Some(addr);
		self
	}

	/// Set the number of worker threads for the server.
	/// This is the number of threads that will be used to process incoming
	/// connections.
	/// Defaults to 1.
	pub fn with_workers(mut self, count: usize) -> Self {
		self.worker_count = count;
		self
	}

	/// Set the keep alive timeout for the server.
	/// Defaults to 5 seconds.
	pub fn with_keep_alive_timeout(mut self, timeout: impl Into<Option<std::time::Duration>>) -> Self {
		self.keep_alive_timeout = timeout.into();
		self
	}

	/// Build the server.
	pub fn build<M>(self, make_service: M) -> Result<Server<M>, Error>
	where
		M: MakeService,
	{
		let bind = self.bind.ok_or(Error::NoBindAddress)?;

		Ok(Server {
			bind,
			#[cfg(feature = "http-tls")]
			insecure_bind: self.insecure_bind,
			#[cfg(feature = "http-tls")]
			tls: self.tls.map(|mut tls| {
				if tls.alpn_protocols.is_empty() {
					tls.alpn_protocols = vec![b"http/1.1".to_vec()];
					if cfg!(feature = "http2") {
						tls.alpn_protocols.push(b"h2".to_vec());
					}
					#[cfg(feature = "http3")]
					if self.quic.is_some() {
						tls.alpn_protocols.push(b"h3".to_vec());
					}
				}

				Arc::new(tls)
			}),
			http1_2: Arc::new(self.http1_2),
			#[cfg(feature = "http3")]
			quic: self.quic,
			make_service,
			backends: Vec::new(),
			handler: None,
			worker_count: self.worker_count,
			keep_alive_timeout: self.keep_alive_timeout,
		})
	}
}
