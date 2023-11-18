use common::context::Context;

pub trait Config {
	fn parse() -> anyhow::Result<Self>
	where
		Self: Sized;

	fn logging(&self) -> &common::config::LoggingConfig;

	fn name(&self) -> &str;

	fn pre_hook(&mut self) -> anyhow::Result<()> {
		Ok(())
	}
}

#[async_trait::async_trait]
pub trait Global<C: Config> {
	async fn new(ctx: Context, config: C) -> anyhow::Result<Self>
	where
		Self: Sized;
}
