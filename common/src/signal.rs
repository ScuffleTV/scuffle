use tokio::{signal::unix::SignalKind, sync::mpsc};

pub struct SignalHandler {
    signal_send: mpsc::Sender<SignalKind>,
    signal_recv: mpsc::Receiver<SignalKind>,
}

impl Default for SignalHandler {
    fn default() -> Self {
        let (signal_send, signal_recv) = mpsc::channel(1);
        Self {
            signal_send,
            signal_recv,
        }
    }
}

impl SignalHandler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_signal(self, kind: SignalKind) -> Self {
        let mut signal = tokio::signal::unix::signal(kind).expect("failed to create signal");

        let send = self.signal_send.clone();
        tokio::spawn(async move {
            loop {
                signal.recv().await;
                if send.send(kind).await.is_err() {
                    break;
                }
            }
        });

        self
    }

    pub async fn recv(&mut self) -> SignalKind {
        self.signal_recv
            .recv()
            .await
            .expect("failed to receive signal")
    }
}

#[cfg(test)]
mod tests;
