use std::sync::Arc;

use async_graphql::extensions::ExtensionContext;
use async_graphql::Context;

use crate::global::GlobalState;

use crate::api::v1::request_context::RequestContext;

pub trait ContextExt {
    fn get_global(&self) -> &Arc<GlobalState>;
    fn get_req_context(&self) -> &RequestContext;
}

impl ContextExt for Context<'_> {
    fn get_global(&self) -> &Arc<GlobalState> {
        self.data_unchecked()
    }

    fn get_req_context(&self) -> &RequestContext {
        self.data_unchecked()
    }
}

impl ContextExt for ExtensionContext<'_> {
    fn get_global(&self) -> &Arc<GlobalState> {
        self.data_unchecked()
    }

    fn get_req_context(&self) -> &RequestContext {
        self.data_unchecked()
    }
}

pub trait RequestExt {
    fn provide_global(self, global: Arc<GlobalState>) -> Self;
    fn provide_context(self, ctx: RequestContext) -> Self;
}

impl RequestExt for async_graphql::Request {
    fn provide_global(self, global: Arc<GlobalState>) -> Self {
        self.data(global)
    }

    fn provide_context(self, ctx: RequestContext) -> Self {
        self.data(ctx)
    }
}

impl RequestExt for async_graphql::Data {
    fn provide_global(mut self, global: Arc<GlobalState>) -> Self {
        self.insert(global);
        self
    }

    fn provide_context(mut self, ctx: RequestContext) -> Self {
        self.insert(ctx);
        self
    }
}
