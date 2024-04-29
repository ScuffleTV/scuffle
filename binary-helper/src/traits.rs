use scuffle_utils::context::Context;

pub trait Config {
	fn parse() -> anyhow::Result<Self>
	where
		Self: Sized;

	fn logging(&self) -> &crate::config::LoggingConfig;

	fn name(&self) -> &str;

	fn pre_hook(&mut self) -> anyhow::Result<()> {
		Ok(())
	}
}

#[allow(async_fn_in_trait)]
pub trait Global<C: Config> {
	async fn new(ctx: Context, config: C) -> anyhow::Result<Self>
	where
		Self: Sized;
}
