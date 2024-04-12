use std::sync::Arc;

use anyhow::Context;
use utils::database::deadpool_postgres::Transaction;

use crate::global::ImageProcessorGlobal;

#[path = "0001_initial.rs"]
mod initial;

#[async_trait::async_trait]
trait Migration<S: ImageProcessorGlobal> {
	fn name(&self) -> &'static str;
	fn version(&self) -> i32;

	async fn up(&self, global: &Arc<S>, tx: &Transaction<'_>) -> anyhow::Result<()>;
	async fn down(&self, global: &Arc<S>, tx: &Transaction<'_>) -> anyhow::Result<()>;
}

const fn migrations<S: ImageProcessorGlobal>() -> &'static [&'static dyn Migration<S>] {
	&[&initial::InitialMigration]
}

#[tracing::instrument(skip(global))]
async fn get_migrations<S: ImageProcessorGlobal>(global: &Arc<S>) -> anyhow::Result<Vec<&'static dyn Migration<S>>> {
	let migrations = migrations::<S>();

	let migration_version = match utils::database::query("SELECT version FROM image_processor_migrations")
		.build_query_single_scalar::<i32>()
		.fetch_one(global.db())
		.await
	{
		Ok(version) => version as usize,
		Err(err) => {
			tracing::info!("Initializing database: {}", err);
			utils::database::query("CREATE TABLE image_processor_migrations (version INTEGER NOT NULL)")
				.build()
				.execute(global.db())
				.await
				.context("Failed to create migration table")?;

			utils::database::query("INSERT INTO image_processor_migrations (version) VALUES (0)")
				.build()
				.execute(global.db())
				.await
				.context("Failed to insert initial migration version")?;

			0
		}
	};

	if migration_version > migrations.len() {
		anyhow::bail!(
			"Database is at version {}, but only {} migrations are available",
			migration_version,
			migrations.len()
		);
	}

	Ok(migrations.iter().skip(migration_version).copied().collect())
}

#[tracing::instrument(skip(global, migration), fields(name = migration.name(), version = migration.version()))]
async fn run_migration<S: ImageProcessorGlobal>(
	global: &Arc<S>,
	migration: &'static dyn Migration<S>,
) -> anyhow::Result<()> {
	tracing::info!("Applying migration");

	let mut client = global.db().get().await.context("Failed to get database connection")?;
	let tx = client.transaction().await.context("Failed to start transaction")?;

	migration.up(global, &tx).await.context("Failed to apply migration")?;

	utils::database::query("UPDATE image_processor_migrations SET version = ")
		.push_bind(migration.version() as i32)
		.build()
		.execute(&tx)
		.await
		.context("Failed to update migration version")?;

	tx.commit().await.context("Failed to commit transaction")?;

	tracing::info!("Migration applied");

	Ok(())
}

#[tracing::instrument(skip(global))]
pub async fn run_migrations<S: ImageProcessorGlobal>(global: &Arc<S>) -> anyhow::Result<()> {
	let migrations = get_migrations(global).await?;

	for migration in migrations {
		run_migration(global, migration).await?;
	}

	Ok(())
}
