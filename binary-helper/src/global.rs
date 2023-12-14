use std::io;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context as _;
use async_nats::ServerAddr;
use common::config::{DatabaseConfig, NatsConfig, RedisConfig};
use fred::interfaces::ClientLike;
use fred::types::ServerConfig;
use rustls::RootCertStore;
use sqlx::postgres::PgConnectOptions;
use sqlx::ConnectOptions;

#[macro_export]
macro_rules! impl_global_traits {
	($struct:ty) => {
		impl common::global::GlobalCtx for $struct {
			#[inline(always)]
			fn ctx(&self) -> &Context {
				&self.ctx
			}
		}

		impl common::global::GlobalNats for $struct {
			#[inline(always)]
			fn nats(&self) -> &async_nats::Client {
				&self.nats
			}

			#[inline(always)]
			fn jetstream(&self) -> &async_nats::jetstream::Context {
				&self.jetstream
			}
		}

		impl common::global::GlobalDb for $struct {
			#[inline(always)]
			fn db(&self) -> &Arc<sqlx::PgPool> {
				&self.db
			}
		}

		impl common::global::GlobalConfig for $struct {}
	};
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
				.add_root_certificates((&tls.ca_cert).into())
				.add_client_certificate((&tls.cert).into(), (&tls.key).into());
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

pub async fn setup_database(config: &DatabaseConfig) -> anyhow::Result<Arc<sqlx::PgPool>> {
	Ok(Arc::new(
		sqlx::PgPool::connect_with(
			PgConnectOptions::from_str(&config.uri)
				.context("failed to parse database uri")?
				.disable_statement_logging()
				.to_owned(),
		)
		.await
		.context("failed to connect to database")?,
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
		let mut cert_store = RootCertStore::empty();

		let ca_cert = tokio::fs::read(&tls.ca_cert).await.context("failed to read redis ca cert")?;
		let cert = tokio::fs::read(&tls.cert).await.context("failed to read redis client cert")?;
		let key = tokio::fs::read(&tls.key)
			.await
			.context("failed to read redis client private key")?;

		let ca_certs =
			rustls_pemfile::certs(&mut io::BufReader::new(io::Cursor::new(ca_cert))).collect::<Result<Vec<_>, _>>()?;

		let key = rustls_pemfile::pkcs8_private_keys(&mut io::BufReader::new(io::Cursor::new(key)))
			.next()
			.ok_or_else(|| anyhow::anyhow!("failed to find private key in redis client private key file"))??
			.into();

		let certs = rustls_pemfile::certs(&mut io::BufReader::new(io::Cursor::new(cert))).collect::<Result<Vec<_>, _>>()?;

		for cert in ca_certs {
			cert_store.add(cert).context("failed to add redis ca cert")?;
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
