use std::{
    fmt::{Display, Formatter},
    future::Future,
    pin::Pin,
    sync::{Arc, Weak},
};

use tokio::{
    sync::{broadcast, oneshot},
    time::Instant,
};

struct RawContext {
    _sender: oneshot::Sender<()>,
    _weak: Weak<()>,
    deadline: Option<Instant>,
    parent: Option<Context>,
    cancel_receiver: broadcast::Receiver<()>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CancelReason {
    Parent,
    Deadline,
    Cancel,
}

impl Display for CancelReason {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parent => write!(f, "Parent"),
            Self::Deadline => write!(f, "Deadline"),
            Self::Cancel => write!(f, "Cancel"),
        }
    }
}

impl RawContext {
    #[must_use]
    fn new() -> (Self, Handler) {
        let (sender, recv) = oneshot::channel();
        let (cancel_sender, cancel_receiver) = broadcast::channel(1);
        let strong = Arc::new(());

        (
            Self {
                _sender: sender,
                deadline: None,
                parent: None,
                cancel_receiver,
                _weak: Arc::downgrade(&strong),
            },
            Handler {
                recv,
                _cancel_sender: cancel_sender,
                _strong: strong,
            },
        )
    }

    #[must_use]
    fn with_deadline(deadline: Instant) -> (Self, Handler) {
        let (mut ctx, handler) = Self::new();
        ctx.deadline = Some(deadline);
        (ctx, handler)
    }

    #[must_use]
    fn with_parent(parent: Context, deadline: Option<Instant>) -> (Self, Handler) {
        let (mut ctx, handler) = Self::new();
        ctx.parent = Some(parent);
        ctx.deadline = deadline;
        (ctx, handler)
    }

    fn done(&self) -> Pin<Box<dyn Future<Output = CancelReason> + '_ + Send>> {
        let mut recv = self.cancel_receiver.resubscribe();
        Box::pin(async move {
            match (&self.parent, self.deadline) {
                (Some(parent), Some(deadline)) => {
                    tokio::select! {
                        _ = parent.done() => CancelReason::Parent,
                        _ = tokio::time::sleep_until(deadline) => CancelReason::Deadline,
                        _ = recv.recv() => CancelReason::Cancel,
                    }
                }
                (Some(parent), None) => {
                    tokio::select! {
                        _ = parent.done() => CancelReason::Parent,
                        _ = recv.recv() => CancelReason::Cancel,
                    }
                }
                (None, Some(deadline)) => {
                    tokio::select! {
                        _ = tokio::time::sleep_until(deadline) => CancelReason::Deadline,
                        _ = recv.recv() => CancelReason::Cancel,
                    }
                }
                (None, None) => {
                    let _ = recv.recv().await;
                    CancelReason::Cancel
                }
            }
        })
    }

    fn is_done(&self) -> bool {
        self._weak.upgrade().is_none()
    }
}

pub struct Handler {
    _strong: Arc<()>,
    recv: oneshot::Receiver<()>,
    _cancel_sender: broadcast::Sender<()>,
}

impl Handler {
    pub async fn done(&mut self) {
        let _ = (&mut self.recv).await;
    }

    pub fn cancel(self) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let recv = self.recv;
        Box::pin(async move {
            let _ = recv.await;
        })
    }
}

#[derive(Clone)]
pub struct Context(Arc<RawContext>);

impl From<RawContext> for Context {
    fn from(ctx: RawContext) -> Self {
        Self(Arc::new(ctx))
    }
}

impl Context {
    pub fn new() -> (Self, Handler) {
        let (ctx, handler) = RawContext::new();
        (ctx.into(), handler)
    }

    pub fn with_deadline(deadline: Instant) -> (Self, Handler) {
        let (ctx, handler) = RawContext::with_deadline(deadline);
        (ctx.into(), handler)
    }

    pub fn with_timeout(timeout: std::time::Duration) -> (Self, Handler) {
        let deadline = Instant::now() + timeout;
        Self::with_deadline(deadline)
    }

    pub fn with_parent(parent: Context, deadline: Option<Instant>) -> (Self, Handler) {
        let (ctx, handler) = RawContext::with_parent(parent, deadline);
        (ctx.into(), handler)
    }

    pub fn done(&self) -> Pin<Box<dyn Future<Output = CancelReason> + '_ + Send>> {
        self.0.done()
    }

    pub fn is_done(&self) -> bool {
        self.0.is_done()
    }
}
