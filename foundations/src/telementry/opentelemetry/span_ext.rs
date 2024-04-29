use opentelemetry::trace::{SpanContext, Status, TraceId};

use super::layer::WithContext;

pub trait OpenTelemetrySpanExt {
    fn link_span(&self, context: SpanContext);
    fn make_root(&self);
    fn set_status(&self, status: Status);
    fn trace_id(&self) -> Option<TraceId>;
}

impl OpenTelemetrySpanExt for tracing::Span {
    fn link_span(&self, context: SpanContext) {
        let mut context = Some(context);
        self.with_subscriber(|(id, dispatch)| {
            let Some(ctx) = dispatch.downcast_ref::<WithContext>() else {
                return;
            };

            ctx.with_context(dispatch, id, |node| {
                if let Some(context) = context.take() {
                    node.follows_from_context(context);
                }
            });
        });
    }

    fn make_root(&self) {
        self.with_subscriber(|(id, dispatch)| {
            let Some(ctx) = dispatch.downcast_ref::<WithContext>() else {
                return;
            };

            ctx.with_context(dispatch, id, |node| {
                node.make_root();
            });
        });
    }

    fn set_status(&self, status: Status) {
        let mut status = Some(status);

        self.with_subscriber(|(id, dispatch)| {
            let Some(ctx) = dispatch.downcast_ref::<WithContext>() else {
                return;
            };

            ctx.with_context(dispatch, id, |node| {
                if let Some(status) = status.take() {
                    node.set_status(status);
                }
            });
        });
    }

    fn trace_id(&self) -> Option<TraceId> {
        let mut trace_id = None;

        self.with_subscriber(|(id, dispatch)| {
            let Some(ctx) = dispatch.downcast_ref::<WithContext>() else {
                return;
            };

            ctx.with_context(dispatch, id, |node| {
                trace_id = Some(node.trace_id());
            });
        });

        trace_id
    }
}
