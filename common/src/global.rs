use std::sync::Arc;

use crate::context::Context;

pub trait GlobalCtx {
	fn ctx(&self) -> &Context;
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
	fn db(&self) -> &Arc<sqlx::PgPool>;
}

pub trait GlobalRedis {
	fn redis(&self) -> &Arc<fred::clients::RedisPool>;
}
