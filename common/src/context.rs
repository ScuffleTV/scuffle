use std::{
    fmt::{Display, Formatter},
    future::Future,
    pin::Pin,
    sync::{Arc, Weak},
};

use tokio::{sync::oneshot, time::Instant};
use tokio_util::sync::{CancellationToken, DropGuard};

struct RawContext {
    _sender: oneshot::Sender<()>,
    _weak: Weak<()>,
    deadline: Option<Instant>,
    parent: Option<Context>,
    cancel_receiver: CancellationToken,
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
        let strong = Arc::new(());
        let token = CancellationToken::new();
        let child = token.child_token();

        (
            Self {
                deadline: None,
                parent: None,
                cancel_receiver: child,
                _sender: sender,
                _weak: Arc::downgrade(&strong),
            },
            Handler {
                recv,
                _token: token.drop_guard(),
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

    fn done(&self) -> Pin<Box<dyn Future<Output = CancelReason> + '_ + Send + Sync>> {
        Box::pin(async move {
            match (&self.parent, self.deadline) {
                (Some(parent), Some(deadline)) => {
                    tokio::select! {
                        _ = parent.done() => CancelReason::Parent,
                        _ = tokio::time::sleep_until(deadline) => CancelReason::Deadline,
                        _ = self.cancel_receiver.cancelled() => CancelReason::Cancel,
                    }
                }
                (Some(parent), None) => {
                    tokio::select! {
                        _ = parent.done() => CancelReason::Parent,
                        _ = self.cancel_receiver.cancelled() => CancelReason::Cancel,
                    }
                }
                (None, Some(deadline)) => {
                    tokio::select! {
                        _ = tokio::time::sleep_until(deadline) => CancelReason::Deadline,
                        _ = self.cancel_receiver.cancelled() => CancelReason::Cancel,
                    }
                }
                (None, None) => {
                    self.cancel_receiver.cancelled().await;
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
    _token: DropGuard,

    recv: oneshot::Receiver<()>,
}

impl Handler {
    pub async fn done(&mut self) {
        let _ = (&mut self.recv).await;
    }

    pub fn cancel(self) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
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

    pub fn done(&self) -> Pin<Box<dyn Future<Output = CancelReason> + '_ + Send + Sync>> {
        self.0.done()
    }

    pub fn is_done(&self) -> bool {
        self.0.is_done()
    }
}
