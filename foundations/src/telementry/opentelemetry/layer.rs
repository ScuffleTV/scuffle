use std::{
    hash::{Hash, Hasher},
    sync::{
        atomic::{AtomicU64, AtomicUsize},
        Arc,
    },
};

use opentelemetry::trace::SpanId;
use opentelemetry_otlp::SpanExporter;
use rand::Rng;
use thread_local::ThreadLocal;
use tracing::{span, Subscriber};
use tracing_subscriber::{registry::LookupSpan, Layer};

use crate::runtime::spawn;

use super::{
    exporter::{BatchExporter, Exporter},
    node::SpanNode,
};

pub(super) struct SpanHolder {
    spans: Vec<SpanNode>,
    max_unprocessed_spans: usize,
    drop_count: usize,
}

impl SpanHolder {
    pub fn new(max_unprocessed_spans: usize) -> Self {
        Self {
            spans: Vec::with_capacity(max_unprocessed_spans),
            max_unprocessed_spans,
            drop_count: 0,
        }
    }

    pub fn push(&mut self, span: SpanNode) {
        if self.spans.len() < self.max_unprocessed_spans {
            self.spans.push(span);
        } else {
            self.drop_count += 1;
        }
    }

    pub fn drain(&mut self, mut new: Vec<SpanNode>) -> Vec<SpanNode> {
        new.clear();
        new.reserve_exact(self.max_unprocessed_spans);

        std::mem::replace(&mut self.spans, new)
    }

    pub fn drop_count(&self) -> usize {
        self.drop_count
    }

    pub fn reset_drop_count(&mut self) {
        self.drop_count = 0;
    }

    pub fn register_drop(&mut self) {
        self.drop_count += 1;
    }
}

pub struct SpanObserverLayer<S> {
    seed: u64,
    config: SpanObserver,
    spans: Arc<ThreadLocal<spin::Mutex<SpanHolder>>>,
    with_context: WithContext,
    _subscriber: std::marker::PhantomData<S>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleResult {
    Sample,
    Dropped,
    NotSampled,
}

pub enum Sampler {
    Always,
    Never,
    TraceIdRatio(f64),
    Custom(Box<dyn ShouldSample>),
}

impl Default for Sampler {
    fn default() -> Self {
        Self::Always
    }
}

impl std::fmt::Debug for Sampler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Sampler::Always => write!(f, "Always"),
            Sampler::Never => write!(f, "Never"),
            Sampler::TraceIdRatio(prob) => write!(f, "TraceIdRatio({})", prob),
            Sampler::Custom(_) => write!(f, "Custom"),
        }
    }
}

impl Sampler {
    pub const fn trace_id_ratio(prob: f64) -> Self {
        Self::TraceIdRatio(prob)
    }

    pub const fn always() -> Self {
        Self::Always
    }

    pub const fn never() -> Self {
        Self::Never
    }

    pub fn custom(s: impl ShouldSample + 'static) -> Self {
        Self::Custom(Box::new(s))
    }

    pub fn cull_children_trace_id_ratio(prob: f64) -> Self {
        let sampler = Sampler::trace_id_ratio(prob);

        Self::function(move |node| {
            if node.is_child() {
                SampleResult::Sample
            } else if node.is_root() {
                if !node.contains_error()
                    && !matches!(sampler.should_sample(node), SampleResult::Sample)
                {
                    node.clear_children();
                }

                SampleResult::Sample
            } else {
                sampler.should_sample(node)
            }
        })
    }

    pub fn function<F: Fn(&mut SpanNode) -> SampleResult + Send + Sync + 'static>(f: F) -> Self {
        Self::custom(SampleFunction::new(f))
    }

    pub fn ratelimit(self, rate: usize, per: std::time::Duration) -> Self {
        Self::custom(RatelimitSampler::new(self, rate, per))
    }
}

pub struct RatelimitSampler {
    parent: Sampler,
    rate: usize,
    per: std::time::Duration,
    base: std::time::Instant,
    last_sample: AtomicU64,
    count: AtomicUsize,
}

pub struct SampleFunction<F>(F);

impl<F: Fn(&mut SpanNode) -> SampleResult + Send + Sync + 'static> SampleFunction<F> {
    pub fn new(f: F) -> Self {
        Self(f)
    }
}

impl<F: Fn(&mut SpanNode) -> SampleResult + Send + Sync + 'static> ShouldSample
    for SampleFunction<F>
{
    fn should_sample(&self, node: &mut SpanNode) -> SampleResult {
        (self.0)(node)
    }
}

impl RatelimitSampler {
    pub fn new(parent: Sampler, rate: usize, per: std::time::Duration) -> Self {
        Self {
            parent,
            rate,
            per,
            base: std::time::Instant::now(),
            last_sample: AtomicU64::new(0),
            count: AtomicUsize::new(0),
        }
    }
}

impl ShouldSample for RatelimitSampler {
    fn should_sample(&self, node: &mut SpanNode) -> SampleResult {
        match self.parent.should_sample(node) {
            SampleResult::Sample => {}
            r => return r,
        };

        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.base)
            + std::time::Duration::from_nanos(
                self.last_sample.load(std::sync::atomic::Ordering::Relaxed),
            );

        if elapsed >= self.per {
            self.last_sample.store(
                now.duration_since(self.base).as_nanos() as u64,
                std::sync::atomic::Ordering::Relaxed,
            );
            self.count.store(0, std::sync::atomic::Ordering::Relaxed);
        }

        let count = self
            .count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        if count < self.rate {
            SampleResult::Sample
        } else {
            SampleResult::Dropped
        }
    }
}

impl ShouldSample for Sampler {
    fn should_sample(&self, node: &mut SpanNode) -> SampleResult {
        match self {
            Sampler::Always => SampleResult::Sample,
            Sampler::Never => SampleResult::NotSampled,
            Sampler::TraceIdRatio(prob) => {
                if prob >= &1.0 {
                    return SampleResult::Sample;
                } else if prob <= &0.0 {
                    return SampleResult::NotSampled;
                }

                let prob_upper_bound = (prob.max(0.0) * (1u64 << 63) as f64) as u64;
                let bytes = node.trace_id().to_bytes();
                let (_, low) = bytes.split_at(8);
                let trace_id_low = u64::from_be_bytes(low.try_into().unwrap());
                let rnd_from_trace_id = trace_id_low >> 1;

                if rnd_from_trace_id < prob_upper_bound {
                    SampleResult::Sample
                } else {
                    SampleResult::NotSampled
                }
            }
            Sampler::Custom(s) => s.should_sample(node),
        }
    }
}

pub trait ShouldSample: Send + Sync {
    fn should_sample(&self, node: &mut SpanNode) -> SampleResult;
}

#[derive(Debug)]
pub struct SpanObserver {
    pub max_unprocessed_spans_per_thread: usize,
    pub sampler: Sampler,
}

impl Default for SpanObserver {
    fn default() -> Self {
        Self {
            max_unprocessed_spans_per_thread: 500,
            sampler: Sampler::Always,
        }
    }
}

impl<S> SpanObserverLayer<S>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    pub fn new(config: SpanObserver, batch_config: BatchExporter, exporter: SpanExporter) -> Self {
        let spans = Arc::new(ThreadLocal::new());

        let exporter = Exporter::new(exporter, batch_config, spans.clone());

        spawn(Box::pin(exporter.run()));

        Self {
            config,
            seed: rand::thread_rng().gen(),
            spans,
            with_context: WithContext(Self::get_context),
            _subscriber: std::marker::PhantomData,
        }
    }

    fn get_context(
        dispatch: &tracing::Dispatch,
        span_id: &span::Id,
        f: &mut dyn FnMut(&mut SpanNode),
    ) {
        let subscriber = dispatch.downcast_ref::<S>().unwrap();
        let span = subscriber.span(span_id).unwrap();

        let mut extensions = span.extensions_mut();
        if let Some(node) = extensions.get_mut::<SpanNode>() {
            f(node);
        }
    }

    fn hash(&self, id: &span::Id) -> SpanId {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.seed.hash(&mut hasher);
        id.hash(&mut hasher);
        SpanId::from_bytes(hasher.finish().to_be_bytes())
    }
}

#[allow(clippy::type_complexity)]
pub(super) struct WithContext(fn(&tracing::Dispatch, &span::Id, f: &mut dyn FnMut(&mut SpanNode)));

impl WithContext {
    pub fn with_context(
        &self,
        dispatch: &tracing::Dispatch,
        span_id: &span::Id,
        mut f: impl FnMut(&mut SpanNode),
    ) {
        (self.0)(dispatch, span_id, &mut f);
    }
}

impl<S> Layer<S> for SpanObserverLayer<S>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(
        &self,
        attrs: &span::Attributes<'_>,
        id: &span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let span = ctx.span(id).unwrap();
        if span.extensions().get::<SpanNode>().is_some() {
            return;
        }

        let mut parent = ctx.current_span().id().cloned();

        let trace_id = parent
            .as_ref()
            .and_then(|id| Some(ctx.span(id)?.extensions().get::<SpanNode>()?.trace_id()));

        if trace_id.is_none() {
            parent = None;
        }

        let root_id = parent.as_ref().and_then(|id| {
            ctx.span(id)?
                .extensions()
                .get::<SpanNode>()?
                .root_id()
                .cloned()
        });

        span.extensions_mut().insert(SpanNode::new(
            id.clone(),
            trace_id,
            self.hash(id),
            parent.map(|id| self.hash(&id)),
            attrs,
            root_id,
        ));
    }

    fn on_close(&self, id: span::Id, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let span = ctx.span(&id).unwrap();

        let mut node = span.extensions_mut().remove::<SpanNode>().unwrap();
        node.close();

        let spans = self.spans.get_or(|| {
            spin::Mutex::new(SpanHolder::new(
                self.config.max_unprocessed_spans_per_thread,
            ))
        });

        match self.config.sampler.should_sample(&mut node) {
            SampleResult::Sample => {
                if node.is_child() {
                    let parent_id = node.root_id().unwrap();
                    let parent = ctx.span(parent_id).unwrap();
                    let mut extensions = parent.extensions_mut();
                    if let Some(parent_node) = extensions.get_mut::<SpanNode>() {
                        parent_node.add_child(node);
                        return;
                    }
                }

                spans.lock().push(node);
            }
            SampleResult::NotSampled => {}
            SampleResult::Dropped => spans.lock().register_drop(),
        }
    }

    fn on_enter(&self, id: &span::Id, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let span = ctx.span(id).unwrap();

        let mut ext = span.extensions_mut();
        if let Some(node) = ext.get_mut::<SpanNode>() {
            node.enter();
        }
    }

    fn on_exit(&self, id: &span::Id, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let span = ctx.span(id).unwrap();

        let mut ext = span.extensions_mut();
        if let Some(node) = ext.get_mut::<SpanNode>() {
            node.exit();
        }
    }

    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let span = ctx.current_span();
        let Some(id) = span.id() else {
            return;
        };

        let span = ctx.span(id).unwrap();

        let mut ext = span.extensions_mut();
        if let Some(node) = ext.get_mut::<SpanNode>() {
            node.event(event);
        }
    }

    fn on_record(
        &self,
        span: &span::Id,
        values: &span::Record<'_>,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let span = ctx.span(span).unwrap();

        let mut ext = span.extensions_mut();
        if let Some(node) = ext.get_mut::<SpanNode>() {
            node.record(values);
        }
    }

    fn on_follows_from(
        &self,
        id: &span::Id,
        follow_id: &span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let span = ctx.span(id).unwrap();
        let Some(follow_span) = ctx.span(follow_id) else {
            return;
        };

        let mut extensions = span.extensions_mut();

        let Some(span_data) = extensions.get_mut::<SpanNode>() else {
            return;
        };

        let follow_span_id = self.hash(follow_id);

        span_data.follows_from(follow_span_id, follow_span.extensions().get::<SpanNode>());
    }

    /// Safety: The lifetime of the with_context is tied to the lifetime of the layer.
    unsafe fn downcast_raw(&self, id: std::any::TypeId) -> Option<*const ()> {
        if id == std::any::TypeId::of::<Self>() {
            return Some(self as *const Self as *const ());
        } else if id == std::any::TypeId::of::<WithContext>() {
            return Some(&self.with_context as *const WithContext as *const ());
        }

        None
    }
}
