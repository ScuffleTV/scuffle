use crate::global::GlobalState;
use anyhow::{anyhow, Result};
use async_stream::stream;
use bytes::Bytes;
use bytesio::{bytesio::BytesIO, bytesio_errors::BytesIOError};
use fred::interfaces::KeysInterface;
use fred::types::{Expiration, SetOptions};
use futures_util::{FutureExt, StreamExt};
use std::{io, sync::Arc};
use tokio::{net::UnixListener, sync::broadcast};
use tokio_util::sync::CancellationToken;

pub fn unix_stream(
    listener: UnixListener,
    buffer_size: usize,
) -> impl futures::Stream<Item = io::Result<Bytes>> {
    stream! {
        let (sock, _) = match listener.accept().await {
            Ok(connection) => connection,
            Err(err) => {
                yield Err(err);
                return;
            }
        };

        let mut bio = BytesIO::with_capacity(sock, buffer_size);

        loop {
            match bio.read().await {
                Ok(bytes) => {
                    yield Ok(bytes.freeze());
                },
                Err(err) => {
                    match err {
                        BytesIOError::ClientClosed => {
                            return;
                        },
                        _ => {
                            yield Err(io::Error::new(io::ErrorKind::UnexpectedEof, anyhow!("failed to read from socket: {}", err)));
                        }
                    }
                }
            }
        }
    }
}

pub struct MultiStream<T: Clone>(broadcast::Sender<Result<T, Arc<io::Error>>>);

pub struct MultiStreamSubscriber<T: Clone>(broadcast::Receiver<Result<T, Arc<io::Error>>>);

impl<T: Clone> MultiStream<T> {
    pub fn new() -> Self {
        Self(broadcast::channel(1024).0)
    }

    pub fn subscribe(&self) -> MultiStreamSubscriber<T> {
        MultiStreamSubscriber(self.0.subscribe())
    }

    pub async fn run(&mut self, stream: impl futures::Stream<Item = io::Result<T>> + Unpin) {
        let mut stream = stream;
        while let Some(data) = stream.next().await {
            let data = match data {
                Ok(data) => Ok(data),
                Err(err) => Err(Arc::new(err)),
            };
            self.0.send(data).ok();
        }
    }
}

impl<T: Clone> MultiStreamSubscriber<T> {
    pub fn into_stream(self) -> impl futures::Stream<Item = io::Result<T>> {
        stream! {
            let mut receiver = self.0;
            while let Ok(data) = receiver.recv().await {
                match data {
                    Ok(data) => yield Ok(data),
                    Err(err) => {
                        yield Err(io::Error::new(err.kind(), anyhow!("multi stream error: {}", err)));
                        return;
                    },
                }
            }
        }
    }
}

pub struct SharedFuture<O, F: futures::Future<Output = O>> {
    inner: F,
    output: Option<Arc<O>>,
}

impl<O, F: futures::Future<Output = O> + Unpin> SharedFuture<O, F> {
    pub fn new(inner: F) -> Self {
        Self {
            inner,
            output: None,
        }
    }
}

impl<O, F: futures::Future<Output = O> + Unpin> futures::Future for SharedFuture<O, F> {
    type Output = Arc<O>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.get_mut();
        if let Some(output) = this.output.as_ref() {
            return std::task::Poll::Ready(output.clone());
        }

        let output = futures::ready!(this.inner.poll_unpin(cx));
        let output = Arc::new(output);
        this.output = Some(output.clone());
        std::task::Poll::Ready(output)
    }
}

pub async fn set_lock(
    global: Arc<GlobalState>,
    key: String,
    req_id: String,
    owned: CancellationToken,
) -> Result<()> {
    loop {
        let have_lock: String = global
            .redis
            .set(
                &key,
                &req_id,
                Some(Expiration::EX(5)),
                Some(SetOptions::NX),
                true,
            )
            .await?;
        if have_lock == req_id {
            break;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    owned.cancel();

    let mut timer = tokio::time::interval(tokio::time::Duration::from_secs(1));
    loop {
        timer.tick().await;

        let lock_owner: String = global
            .redis
            .set(
                &key,
                &req_id,
                Some(Expiration::EX(5)),
                Some(SetOptions::XX),
                true,
            )
            .await?;
        if lock_owner != req_id {
            return Err(anyhow!("lost lock"));
        }
    }
}

pub async fn release_lock(global: &Arc<GlobalState>, key: &str, request_id: &str) -> Result<()> {
    let lock_owner: String = global
        .redis
        .set(
            key,
            request_id,
            Some(Expiration::EX(5)),
            Some(SetOptions::XX),
            true,
        )
        .await?;

    if lock_owner == request_id {
        global.redis.del(key).await?;
    }

    Ok(())
}
