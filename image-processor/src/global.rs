use std::collections::HashMap;

use anyhow::Context;
use bson::oid::ObjectId;
use scuffle_foundations::BootstrapResult;

use crate::config::ImageProcessorConfig;
use crate::database::Job;
use crate::disk::public_http::PUBLIC_HTTP_DISK_NAME;
use crate::disk::{build_disk, AnyDisk, Disk};
use crate::event_queue::{build_event_queue, AnyEventQueue, EventQueue};

pub struct Global {
	worker_id: ObjectId,
	config: ImageProcessorConfig,
	client: mongodb::Client,
	database: mongodb::Database,
	disks: HashMap<String, AnyDisk>,
	event_queues: HashMap<String, AnyEventQueue>,
}

impl Global {
	pub async fn new(config: ImageProcessorConfig) -> BootstrapResult<Self> {
		tracing::debug!("setting up mongo client");

		let client = mongodb::Client::with_uri_str(&config.database.uri).await.context("mongodb")?;
		let Some(database) = client.default_database() else {
			anyhow::bail!("no default database")
		};

		tracing::debug!("setting up job collection");

		Job::setup_collection(&database).await.context("setup job collection")?;

		tracing::debug!("setting up disks and event queues");

		let mut disks = HashMap::new();

		for disk in &config.disks {
			let disk = build_disk(disk).await.context("disk")?;
			let name = disk.name().to_string();
			if disks.insert(name.clone(), disk).is_some() {
				anyhow::bail!("duplicate disk name: {name}");
			}
		}

		if config.disks.is_empty() {
			tracing::warn!("no disks configured");
		}

		let mut event_queues = HashMap::new();

		for event_queue in &config.event_queues {
			let event_queue = build_event_queue(event_queue).await.context("event queue")?;
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

	pub fn disk(&self, name: &str) -> Option<&AnyDisk> {
		self.disks.get(name)
	}

	pub fn event_queue(&self, name: &str) -> Option<&AnyEventQueue> {
		self.event_queues.get(name)
	}

	pub fn public_http_disk(&self) -> Option<&AnyDisk> {
		self.disk(PUBLIC_HTTP_DISK_NAME)
	}

	pub fn database(&self) -> &mongodb::Database {
		&self.database
	}
}
