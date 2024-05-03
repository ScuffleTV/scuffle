mod exporter;
mod layer;
mod node;
mod span_ext;

pub use exporter::BatchExporter;
pub use layer::{RatelimitSampler, SampleFunction, SampleResult, Sampler, ShouldSample, SpanObserver, SpanObserverLayer};
pub use node::SpanNode;
use opentelemetry_otlp::SpanExporter;
pub use span_ext::OpenTelemetrySpanExt;

pub fn layer<S>(span_observer: SpanObserver, batch_config: BatchExporter, exporter: SpanExporter) -> SpanObserverLayer<S>
where
	S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
	SpanObserverLayer::new(span_observer, batch_config, exporter)
}

pub fn complex_rate_sampler(
	head_rate: f64,
	tail_rate: Option<f64>,
	error_rate: Option<f64>,
	sample_on_error: bool,
) -> Sampler {
	Sampler::function(move |node| {
		let tail_rate = tail_rate.unwrap_or(head_rate);
		let rate_to_use = if node.contains_error() {
			error_rate.unwrap_or(if node.is_child() { tail_rate } else { head_rate })
		} else {
			head_rate
		};

		if node.is_child() {
			if sample_on_error {
				// always sample children because we dont know if there are errors yet.
				SampleResult::Sample
			} else {
				Sampler::TraceIdRatio(rate_to_use).should_sample(node)
			}
		} else if node.is_root() {
			match Sampler::TraceIdRatio(rate_to_use).should_sample(node) {
				SampleResult::Sample if (!sample_on_error || !node.contains_error()) && tail_rate != head_rate => {
					let should_sample_children = Sampler::TraceIdRatio(tail_rate).should_sample(node);
					if should_sample_children != SampleResult::Sample {
						node.clear_children();
					}

					SampleResult::Sample
				}
				r => r,
			}
		} else {
			Sampler::TraceIdRatio(head_rate).should_sample(node)
		}
	})
}
