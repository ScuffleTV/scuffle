use std::io;
use std::sync::{Arc, Weak};
use std::time::Duration;

use anyhow::Context as _;
use async_nats::ServerAddr;
use bytes::Bytes;
use fred::interfaces::ClientLike;
use fred::types::ServerConfig;
use hyper::StatusCode;
use rustls::RootCertStore;
use utils::database::deadpool_postgres::{ManagerConfig, PoolConfig, RecyclingMethod, Runtime};
use utils::database::tokio_postgres::NoTls;
use utils::database::Pool;
use utils::http::RouteError;

use crate::config::{DatabaseConfig, NatsConfig, RedisConfig};

#[macro_export]
macro_rules! impl_global_traits {
	($struct:ty) => {
		impl binary_helper::global::GlobalCtx for $struct {
			#[inline(always)]
			fn ctx(&self) -> &Context {
				&self.ctx
			}
		}

		impl binary_helper::global::GlobalNats for $struct {
			#[inline(always)]
			fn nats(&self) -> &async_nats::Client {
				&self.nats
			}

			#[inline(always)]
			fn jetstream(&self) -> &async_nats::jetstream::Context {
				&self.jetstream
			}
		}

		impl binary_helper::global::GlobalDb for $struct {
			#[inline(always)]
			fn db(&self) -> &Arc<utils::database::Pool> {
				&self.db
			}
		}

		impl binary_helper::global::GlobalConfig for $struct {}
	};
}

pub trait GlobalCtx {
	fn ctx(&self) -> &utils::context::Context;
}

pub trait GlobalConfig {
	#[inline(always)]
	fn config<C>(&self) -> &C
	where
		Self: GlobalConfigProvider<C>,
	{
		GlobalConfigProvider::provide_config(self)
	}
}

pub trait GlobalConfigProvider<C> {
	fn provide_config(&self) -> &C;
}

pub trait GlobalNats {
	fn nats(&self) -> &async_nats::Client;
	fn jetstream(&self) -> &async_nats::jetstream::Context;
}

pub trait GlobalDb {
	fn db(&self) -> &Arc<deadpool_postgres::Pool>;
}

pub trait GlobalRedis {
	fn redis(&self) -> &Arc<fred::clients::RedisPool>;
}

pub async fn setup_nats(
	name: &str,
	config: &NatsConfig,
) -> anyhow::Result<(async_nats::Client, async_nats::jetstream::Context)> {
	let nats = {
		let mut options = async_nats::ConnectOptions::new()
			.connection_timeout(Duration::from_secs(5))
			.name(name)
			.retry_on_initial_connect();

		if let Some(user) = &config.username {
			options = options.user_and_password(user.clone(), config.password.clone().unwrap_or_default())
		} else if let Some(token) = &config.token {
			options = options.token(token.clone())
		}

		if let Some(tls) = &config.tls {
			options = options
				.require_tls(true)
				.add_client_certificate((&tls.cert).into(), (&tls.key).into());

			if let Some(ca_cert) = &tls.ca_cert {
				options = options.add_root_certificates(ca_cert.into())
			}
		}

		options
			.connect(
				config
					.servers
					.iter()
					.map(|s| s.parse::<ServerAddr>())
					.collect::<Result<Vec<_>, _>>()
					.context("failed to parse nats server addresses")?,
			)
			.await
			.context("failed to connect to nats")?
	};

	let jetstream = async_nats::jetstream::new(nats.clone());

	Ok((nats, jetstream))
}

pub async fn setup_database(config: &DatabaseConfig) -> anyhow::Result<Arc<utils::database::Pool>> {
	let mut pg_config = config
		.uri
		.parse::<utils::database::tokio_postgres::Config>()
		.context("invalid database uri")?;

	pg_config.ssl_mode(if config.tls.is_some() {
		utils::database::tokio_postgres::config::SslMode::Require
	} else {
		utils::database::tokio_postgres::config::SslMode::Disable
	});

	let manager = if let Some(tls) = &config.tls {
		let cert = tokio::fs::read(&tls.cert).await.context("failed to read redis client cert")?;
		let key = tokio::fs::read(&tls.key)
			.await
			.context("failed to read redis client private key")?;

		let key = rustls_pemfile::pkcs8_private_keys(&mut io::BufReader::new(io::Cursor::new(key)))
			.next()
			.ok_or_else(|| anyhow::anyhow!("failed to find private key in redis client private key file"))??
			.into();

		let certs = rustls_pemfile::certs(&mut io::BufReader::new(io::Cursor::new(cert))).collect::<Result<Vec<_>, _>>()?;

		let mut cert_store = RootCertStore::empty();
		if let Some(ca_cert) = &tls.ca_cert {
			let ca_cert = tokio::fs::read(ca_cert).await.context("failed to read redis ca cert")?;
			let ca_certs =
				rustls_pemfile::certs(&mut io::BufReader::new(io::Cursor::new(ca_cert))).collect::<Result<Vec<_>, _>>()?;
			for cert in ca_certs {
				cert_store.add(cert).context("failed to add redis ca cert")?;
			}
		}

		let tls = rustls::ClientConfig::builder()
			.with_root_certificates(cert_store)
			.with_client_auth_cert(certs, key)
			.context("failed to create redis tls config")?;

		utils::database::deadpool_postgres::Manager::from_config(
			pg_config,
			tokio_postgres_rustls::MakeRustlsConnect::new(tls),
			ManagerConfig {
				recycling_method: RecyclingMethod::Fast,
			},
		)
	} else {
		utils::database::deadpool_postgres::Manager::from_config(
			pg_config,
			NoTls,
			ManagerConfig {
				recycling_method: RecyclingMethod::Fast,
			},
		)
	};

	Ok(Arc::new(
		Pool::builder(manager)
			.config(PoolConfig::default())
			.runtime(Runtime::Tokio1)
			.build()
			.context("failed to create database pool")?,
	))
}

pub async fn setup_redis(config: &RedisConfig) -> anyhow::Result<Arc<fred::clients::RedisPool>> {
	let hosts = config
		.addresses
		.iter()
		.map(|host| {
			let mut server = fred::types::Server::try_from(host.as_str()).context("failed to parse redis server address")?;
			if let Some(tls) = &config.tls {
				server.tls_server_name = tls.domain.as_ref().map(|d| d.into());
			}

			Ok(server)
		})
		.collect::<anyhow::Result<Vec<_>>>()?;

	let server = if let Some(sentinel) = &config.sentinel {
		ServerConfig::Sentinel {
			hosts,
			service_name: sentinel.service_name.clone(),
		}
	} else if hosts.len() == 1 {
		ServerConfig::Centralized {
			server: hosts.into_iter().next().unwrap(),
		}
	} else {
		ServerConfig::Clustered { hosts }
	};

	let tls = if let Some(tls) = &config.tls {
		let cert = tokio::fs::read(&tls.cert).await.context("failed to read redis client cert")?;
		let key = tokio::fs::read(&tls.key)
			.await
			.context("failed to read redis client private key")?;

		let key = rustls_pemfile::pkcs8_private_keys(&mut io::BufReader::new(io::Cursor::new(key)))
			.next()
			.ok_or_else(|| anyhow::anyhow!("failed to find private key in redis client private key file"))??
			.into();

		let certs = rustls_pemfile::certs(&mut io::BufReader::new(io::Cursor::new(cert))).collect::<Result<Vec<_>, _>>()?;

		let mut cert_store = RootCertStore::empty();
		if let Some(ca_cert) = &tls.ca_cert {
			let ca_cert = tokio::fs::read(ca_cert).await.context("failed to read redis ca cert")?;
			let ca_certs =
				rustls_pemfile::certs(&mut io::BufReader::new(io::Cursor::new(ca_cert))).collect::<Result<Vec<_>, _>>()?;
			for cert in ca_certs {
				cert_store.add(cert).context("failed to add redis ca cert")?;
			}
		}

		Some(fred::types::TlsConfig::from(fred::types::TlsConnector::from(
			rustls::ClientConfig::builder()
				.with_root_certificates(cert_store)
				.with_client_auth_cert(certs, key)
				.context("failed to create redis tls config")?,
		)))
	} else {
		None
	};

	let redis = Arc::new(
		fred::clients::RedisPool::new(
			fred::types::RedisConfig {
				database: Some(config.database),
				password: config.password.clone(),
				username: config.username.clone(),
				server,
				tls,
				..Default::default()
			},
			None,
			None,
			None,
			config.pool_size,
		)
		.context("failed to create redis pool")?,
	);

	redis.connect();
	redis.wait_for_connect().await.context("failed to connect to redis")?;

	Ok(redis)
}

pub trait RequestGlobalExt<E> {
	fn get_global<G: Sync + Send + 'static, B: From<Bytes>>(&self) -> std::result::Result<Arc<G>, RouteError<E, B>>;
}

impl<E, B> RequestGlobalExt<E> for hyper::Request<B> {
	fn get_global<G: Sync + Send + 'static, B2: From<Bytes>>(&self) -> std::result::Result<Arc<G>, RouteError<E, B2>> {
		Ok(self
			.extensions()
			.get::<Weak<G>>()
			.expect("global state not set")
			.upgrade()
			.ok_or((StatusCode::INTERNAL_SERVER_ERROR, "failed to upgrade global state"))?)
	}
}
