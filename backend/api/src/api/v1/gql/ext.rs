use std::sync::Arc;

use async_graphql::Context;

use crate::global::GlobalState;

use super::request_context::RequestContext;

pub trait ContextExt {
    fn get_global(&self) -> &Arc<GlobalState>;
    fn get_session(&self) -> &Arc<RequestContext>;
}

impl ContextExt for Context<'_> {
    fn get_global(&self) -> &Arc<GlobalState> {
        self.data_unchecked::<Arc<GlobalState>>()
    }

    fn get_session(&self) -> &Arc<RequestContext> {
        self.data_unchecked::<Arc<RequestContext>>()
    }
}

pub trait RequestExt {
    fn provide_global(self, global: Arc<GlobalState>) -> Self;
    fn provide_context(self, ctx: Arc<RequestContext>) -> Self;
}

impl RequestExt for async_graphql::Request {
    fn provide_global(self, global: Arc<GlobalState>) -> Self {
        self.data(global)
    }

    fn provide_context(self, ctx: Arc<RequestContext>) -> Self {
        self.data(ctx)
    }
}

impl RequestExt for async_graphql::Data {
    fn provide_global(mut self, global: Arc<GlobalState>) -> Self {
        self.insert(global);
        self
    }

    fn provide_context(mut self, ctx: Arc<RequestContext>) -> Self {
        self.insert(ctx);
        self
    }
}
