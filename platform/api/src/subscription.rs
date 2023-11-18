use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use async_nats::Message;
use common::context::Context;
use tokio::select;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex};
use tokio_stream::{StreamExt, StreamMap, StreamNotifyClose};
use tracing::{debug, error, warn};

#[derive(thiserror::Error, Debug)]
pub enum SubscriptionManagerError {
	#[error("subscribe error: {0}")]
	Subscribe(#[from] async_nats::SubscribeError),
	#[error("unsubscribe error: {0}")]
	Unsubscribe(#[from] async_nats::UnsubscribeError),
	#[error("send error: {0}")]
	Send(#[from] mpsc::error::SendError<Event>),
	#[error("receive error: {0}")]
	Receive(#[from] oneshot::error::RecvError),
}

#[derive(Debug)]
pub enum Event {
	Subscribe {
		topic: String,
		tx: oneshot::Sender<broadcast::Receiver<Message>>,
	},
	Unsubscribe {
		topic: String,
	},
}

pub struct SubscriptionManager {
	events_tx: mpsc::UnboundedSender<Event>,
	events_rx: Mutex<mpsc::UnboundedReceiver<Event>>,
}

impl Default for SubscriptionManager {
	fn default() -> Self {
		// Only one value is needed in the channel.
		// This is a way to get around we cannot await in a drop.
		let (events_tx, events_rx) = mpsc::unbounded_channel();

		Self {
			events_rx: Mutex::new(events_rx),
			events_tx,
		}
	}
}

pub struct SubscriberReceiver<'a> {
	topic: String,
	rx: broadcast::Receiver<Message>,
	manager: &'a SubscriptionManager,
}

impl Deref for SubscriberReceiver<'_> {
	type Target = broadcast::Receiver<Message>;

	fn deref(&self) -> &Self::Target {
		&self.rx
	}
}

impl DerefMut for SubscriberReceiver<'_> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.rx
	}
}

impl SubscriptionManager {
	pub async fn run(&self, ctx: Context, nats: async_nats::Client) -> Result<(), SubscriptionManagerError> {
		let mut topics = HashMap::<String, broadcast::Sender<Message>>::new();
		let mut subs = StreamMap::new();

		let mut events_rx = self.events_rx.lock().await;

		loop {
			select! {
				event = events_rx.recv() => {
					debug!("received event: {:?}", event);

					match event.unwrap() {
						Event::Subscribe { topic, tx } => {
							match topics.get(&topic) {
								Some(broadcast) => {
									// TODO: Handle error?
									tx.send(broadcast.subscribe()).ok();
								},
								None => {
									let (btx, rx) = broadcast::channel(16);
									if tx.send(rx).is_err() {
										// TODO: Handle error?
										warn!("failed to send broadcast receiver to subscriber");
										continue;
									}

									debug!("subscribing to topic: {}", topic);
									let sub = nats.subscribe(topic.clone()).await?;

									topics.insert(topic.clone(), btx);
									subs.insert(topic, StreamNotifyClose::new(sub));
									debug!("topics: {:?}", topics);
								}
							};
						}
						Event::Unsubscribe { topic } => {
							debug!("received unsubscribe event for topic: {}", topic);
							if let Some(btx) = topics.get_mut(&topic) {
								if btx.receiver_count() == 0 {
									topics.remove(&topic);
									if let Some(Some(mut sub)) = subs.remove(&topic).map(|s| s.into_inner()) {
										sub.unsubscribe().await?;
									}
								}
							}

							if topics.is_empty() && ctx.is_done() {
								break;
							}
						}
					}
				}
				Some((topic, message)) = subs.next() => {
					match message {
						Some(message) => {
							debug!("received nats message: {:?}", message);

							let Some(subs) = topics.get(&topic) else {
								debug!("received message for unsubscribed topic: {}", topic);
								continue;
							};

							// TODO: Handle error?
							if let Err(e) = subs.send(message) {
								error!("failed to send message to subscribers: {e}");
							}
						},
						None => {
							// nats subscriber closed
							topics.remove(&topic);
						}
					}
				}
			}
		}

		Ok(())
	}

	pub async fn subscribe(&self, topic: impl ToString) -> Result<SubscriberReceiver<'_>, SubscriptionManagerError> {
		let (tx, rx) = oneshot::channel();

		self.events_tx.send(Event::Subscribe {
			topic: topic.to_string(),
			tx,
		})?;

		let rx = rx.await?;

		Ok(SubscriberReceiver {
			topic: topic.to_string(),
			rx,
			manager: self,
		})
	}
}

impl Drop for SubscriberReceiver<'_> {
	fn drop(&mut self) {
		self.manager
			.events_tx
			.send(Event::Unsubscribe {
				topic: self.topic.clone(),
			})
			.ok();
	}
}
