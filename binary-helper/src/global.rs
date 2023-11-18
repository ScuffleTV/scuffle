use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context as _;
use async_nats::ServerAddr;
use common::config::{DatabaseConfig, NatsConfig};
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
