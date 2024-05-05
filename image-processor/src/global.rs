use std::collections::HashMap;

use anyhow::Context;
use bson::oid::ObjectId;
use scuffle_foundations::telemetry::server::HealthCheck;
use scuffle_foundations::BootstrapResult;

use crate::config::ImageProcessorConfig;
use crate::database::Job;
use crate::disk::public_http::PUBLIC_HTTP_DRIVE_NAME;
use crate::disk::{build_drive, AnyDrive, Drive};
use crate::event_queue::{build_event_queue, AnyEventQueue, EventQueue};

pub struct Global {
	worker_id: ObjectId,
	config: ImageProcessorConfig,
	client: mongodb::Client,
	database: mongodb::Database,
	disks: HashMap<String, AnyDrive>,
	event_queues: HashMap<String, AnyEventQueue>,
}

impl Global {
	pub async fn new(config: ImageProcessorConfig) -> BootstrapResult<Self> {
		const DEFAULT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);
		tracing::debug!("setting up mongo client");

		let client = tokio::time::timeout(DEFAULT_TIMEOUT, mongodb::Client::with_uri_str(&config.database.uri))
			.await
			.context("mongodb timeout")?
			.context("mongodb")?;
		let Some(database) = client.default_database() else {
			anyhow::bail!("no default database")
		};

		tracing::debug!("setting up job collection");

		tokio::time::timeout(DEFAULT_TIMEOUT, Job::setup_collection(&database))
			.await
			.context("job collection timeout")?
			.context("job collection")?;

		tracing::debug!("setting up disks and event queues");

		let mut disks = HashMap::new();

		for disk in &config.drives {
			let disk = tokio::time::timeout(DEFAULT_TIMEOUT, build_drive(disk))
				.await
				.context("disk timeout")?
				.context("disk")?;

			let name = disk.name().to_string();
			if disks.insert(name.clone(), disk).is_some() {
				anyhow::bail!("duplicate disk name: {name}");
			}
		}

		if config.drives.is_empty() {
			tracing::warn!("no disks configured");
		}

		let mut event_queues = HashMap::new();

		for event_queue in &config.event_queues {
			let event_queue = tokio::time::timeout(DEFAULT_TIMEOUT, build_event_queue(event_queue))
				.await
				.context("event queue timeout")?
				.context("event queue")?;

			let name = event_queue.name().to_string();
			if event_queues.insert(name.clone(), event_queue).is_some() {
				anyhow::bail!("duplicate event queue name: {name}");
			}
		}

		if config.event_queues.is_empty() {
			tracing::warn!("no event queues configured");
		}

		Ok(Self {
			worker_id: ObjectId::new(),
			config,
			client,
			database,
			disks,
			event_queues,
		})
	}

	pub fn worker_id(&self) -> ObjectId {
		self.worker_id
	}

	pub fn config(&self) -> &ImageProcessorConfig {
		&self.config
	}

	pub fn drive(&self, name: &str) -> Option<&AnyDrive> {
		self.disks.get(name)
	}

	pub fn drives(&self) -> &HashMap<String, AnyDrive> {
		&self.disks
	}

	pub fn event_queues(&self) -> &HashMap<String, AnyEventQueue> {
		&self.event_queues
	}

	pub fn event_queue(&self, name: &str) -> Option<&AnyEventQueue> {
		self.event_queues.get(name)
	}

	pub fn public_http_drive(&self) -> Option<&AnyDrive> {
		self.drive(PUBLIC_HTTP_DRIVE_NAME)
	}

	pub fn database(&self) -> &mongodb::Database {
		&self.database
	}
}

impl HealthCheck for Global {
	fn check(&self) -> std::pin::Pin<Box<dyn futures::prelude::Future<Output = bool> + Send + '_>> {
		Box::pin(async {
			if let Err(err) = self.database().run_command(bson::doc! { "ping": 1 }, None).await {
				tracing::error!("database ping failed: {err}");
				return false;
			}

			for disk in self.drive().values() {
				if !disk.healthy().await {
					tracing::error!(name = %disk.name(), "disk check failed");
					return false;
				}
			}

			for event_queue in self.event_queues().values() {
				if !event_queue.healthy().await {
					tracing::error!(name = %event_queue.name(), "event queue check failed");
					return false;
				}
			}

			true
		})
	}
}
