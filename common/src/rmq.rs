use std::{
    sync::{atomic::AtomicUsize, Arc},
    time::Duration,
};

use anyhow::{anyhow, Result};
use arc_swap::ArcSwap;
use async_stream::stream;
use futures::{Stream, StreamExt};
use lapin::{
    options::BasicConsumeOptions, topology::TopologyDefinition, types::FieldTable, Channel,
    Connection, ConnectionProperties,
};
use tokio::sync::{broadcast, mpsc, Mutex};
use tracing::{info_span, Instrument};

use crate::prelude::FutureTimeout;

pub struct ConnectionPool {
    uri: String,
    timeout: Duration,
    properties: ConnectionProperties,
    error_queue: mpsc::Sender<usize>,
    error_queue_rx: Mutex<mpsc::Receiver<usize>>,
    new_connection_waker: broadcast::Sender<()>,
    connections: Vec<ArcSwap<Connection>>,
    aquire_idx: AtomicUsize,
}

impl ConnectionPool {
    pub async fn connect(
        uri: String,
        properties: ConnectionProperties,
        timeout: Duration,
        pool_size: usize,
    ) -> Result<Self> {
        let connections = Vec::with_capacity(pool_size);
        let (tx, rx) = mpsc::channel(pool_size);

        let mut pool = Self {
            uri,
            properties,
            timeout,
            connections,
            error_queue: tx,
            error_queue_rx: Mutex::new(rx),
            new_connection_waker: broadcast::channel(1).0,
            aquire_idx: AtomicUsize::new(0),
        };

        for i in 0..pool_size {
            let conn = pool.new_connection(i, None).await?;
            pool.connections.push(ArcSwap::from(Arc::new(conn)));
        }

        Ok(pool)
    }

    pub async fn handle_reconnects(&self) -> Result<()> {
        loop {
            let idx = self
                .error_queue_rx
                .lock()
                .await
                .recv()
                .await
                .expect("error queue closed");
            let conn = async {
                loop {
                    let conn = match self
                        .new_connection(idx, Some(self.connections[idx].load().topology()))
                        .await
                    {
                        Ok(conn) => conn,
                        Err(err) => {
                            tracing::error!("failed to reconnect: {}", err);
                            tokio::time::sleep(Duration::from_secs(1)).await;
                            continue;
                        }
                    };

                    tracing::info!("reconnected to rabbitmq");
                    break conn;
                }
            }
            .instrument(info_span!("reconnect rmq", idx))
            .timeout(self.timeout)
            .await?;

            self.connections[idx].store(Arc::new(conn));
            self.new_connection_waker.send(()).ok();
        }
    }

    pub async fn new_connection(
        &self,
        idx: usize,
        topology: Option<TopologyDefinition>,
    ) -> Result<Connection> {
        let conn = Connection::connect(&self.uri, self.properties.clone())
            .timeout(self.timeout)
            .await??;

        if let Some(topology) = topology {
            conn.restore(topology).await?;
        }

        let sender = self.error_queue.clone();
        conn.on_error(move |e| {
            tracing::error!("rabbitmq error: {:?}", e);

            if let Err(err) = sender.try_send(idx) {
                tracing::error!("failed to reload connection: {}", err);
            }
        });

        Ok(conn)
    }

    pub fn basic_consume(
        &self,
        queue_name: impl ToString,
        connection_name: impl ToString,
        options: BasicConsumeOptions,
        table: FieldTable,
    ) -> impl Stream<Item = Result<lapin::message::Delivery>> + '_ {
        let queue_name = queue_name.to_string();
        let connection_name = connection_name.to_string();

        stream!({
            'connection_loop: loop {
                let channel = self.aquire().await?;
                let mut consumer = channel
                    .basic_consume(&queue_name, &connection_name, options, table.clone())
                    .await?;
                loop {
                    let m = consumer.next().await;
                    match m {
                        Some(Ok(m)) => {
                            yield Ok(m);
                        }
                        Some(Err(e)) => match e {
                            lapin::Error::IOError(e) => {
                                if e.kind() == std::io::ErrorKind::ConnectionReset {
                                    continue 'connection_loop;
                                }
                            }
                            _ => {
                                yield Err(anyhow!("failed to get message: {}", e));
                            }
                        },
                        None => {
                            continue 'connection_loop;
                        }
                    }
                }
            }
        })
    }

    pub async fn aquire(&self) -> Result<Channel> {
        let mut done = false;
        loop {
            let mut conn = None;
            let start_idx = self
                .aquire_idx
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                % self.connections.len();
            for c in self.connections[start_idx..]
                .iter()
                .chain(self.connections[..start_idx].iter())
            {
                let loaded = c.load();
                if loaded.status().connected() {
                    conn = Some(loaded.clone());
                    break;
                }
            }

            if let Some(conn) = conn {
                let channel = conn.create_channel().await?;
                return Ok(channel);
            }

            if done {
                return Err(anyhow!("no connections available"));
            }

            done = true;
            self.new_connection_waker
                .subscribe()
                .recv()
                .timeout(self.timeout)
                .await??;
        }
    }
}
