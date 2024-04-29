use scuffle_utils::context::Context;

use crate::config::ImageProcessorConfig;

pub struct ImageProcessorGlobalImpl {
	ctx: Context,
	config: ImageProcessorConfig,
	http_client: reqwest::Client,
}

pub trait ImageProcessorGlobal: Send + Sync + 'static {
	fn ctx(&self) -> &Context;
	fn config(&self) -> &ImageProcessorConfig;
	fn http_client(&self) -> &reqwest::Client;
}

impl ImageProcessorGlobal for ImageProcessorGlobalImpl {
	fn ctx(&self) -> &Context {
		&self.ctx
	}

	fn config(&self) -> &ImageProcessorConfig {
		&self.config
	}

	fn http_client(&self) -> &reqwest::Client {
		&self.http_client
	}
}
