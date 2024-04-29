use futures::FutureExt;
use tokio::signal::unix::{Signal, SignalKind};

#[derive(Default)]
pub struct SignalHandler {
	signals: Vec<(SignalKind, Signal)>,
}

impl SignalHandler {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn with_signal(&mut self, kind: SignalKind) -> &mut Self {
		if self.signals.iter().any(|(k, _)| k == &kind) {
			return self;
		}

		let signal = tokio::signal::unix::signal(kind).expect("failed to create signal");

		self.signals.push((kind, signal));

		self
	}

	pub async fn recv(&mut self) -> Option<SignalKind> {
		if self.signals.is_empty() {
			return None;
		}

		let (item, _, _) = futures::future::select_all(
			self.signals
				.iter_mut()
				.map(|(kind, signal)| Box::pin(signal.recv().map(|_| *kind))),
		)
		.await;

		Some(item)
	}
}
