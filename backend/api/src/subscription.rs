use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use anyhow::Result;
use common::context::Context;
use fred::{clients::SubscriberClient, prelude::PubsubInterface, types::RedisValue};
use tokio::{
    select,
    sync::{broadcast, mpsc, oneshot, Mutex},
};

#[derive(Debug)]
enum Event {
    Subscribe {
        topic: String,
        tx: oneshot::Sender<broadcast::Receiver<RedisValue>>,
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
    rx: broadcast::Receiver<RedisValue>,
    manager: &'a SubscriptionManager,
}

impl Deref for SubscriberReceiver<'_> {
    type Target = broadcast::Receiver<RedisValue>;

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
    pub async fn run(&self, ctx: Context, redis: SubscriberClient) -> Result<()> {
        let mut handle = redis.manage_subscriptions();

        let mut topics = HashMap::<String, broadcast::Sender<RedisValue>>::new();

        let mut events_rx = self.events_rx.lock().await;

        let mut messages = redis.on_message();

        loop {
            select! {
                event = events_rx.recv() => {
                    match event.unwrap() {
                        Event::Subscribe { topic, tx } => {
                            let topic = topic.to_lowercase();

                            match topics.get(&topic) {
                                Some(broadcast) => {
                                    tx.send(broadcast.subscribe()).ok();
                                },
                                None => {
                                    let (btx, rx) = broadcast::channel(16);
                                    if tx.send(rx).is_err() {
                                        continue;
                                    }

                                    topics.insert(topic.clone(), btx);

                                    redis.subscribe(&topic).await?;
                                }
                            };
                        }
                        Event::Unsubscribe { topic } => {
                            if let Some(btx) = topics.get_mut(&topic) {
                                if btx.receiver_count() == 0 {
                                    topics.remove(&topic);
                                    redis.unsubscribe(&topic).await?;
                                }
                            }

                            if topics.is_empty() && ctx.is_done() {
                                break;
                            }
                        }
                    }
                }
                message = messages.recv() => {
                    let message = message.unwrap();

                    let topic = message.channel.to_string().to_lowercase();

                    let Some(subs) = topics.get(&topic) else {
                        continue;
                    };

                    subs.send(message.value).ok();
                }
                r = &mut handle => {
                    r?;
                    break;
                }
                _ = ctx.done() => {
                    break;
                }
            }
        }

        Ok(())
    }

    pub async fn subscribe(&self, topic: impl ToString) -> Result<SubscriberReceiver<'_>> {
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
