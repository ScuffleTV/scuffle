use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::Arc;
use std::task::Poll;

use tokio_util::sync::{CancellationToken, WaitForCancellationFutureOwned};

#[derive(Debug)]
struct ContextTracker(Arc<ContextTrackerInner>);

impl Drop for ContextTracker {
    fn drop(&mut self) {
        if self
            .active_count
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed)
            == 1
            && self.stopped.load(std::sync::atomic::Ordering::Relaxed)
        {
            self.notify.notify_waiters();
        }
    }
}

impl Clone for ContextTracker {
    fn clone(&self) -> Self {
        self.active_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self(self.0.clone())
    }
}

impl std::ops::Deref for ContextTracker {
    type Target = ContextTrackerInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
struct ContextTrackerInner {
    stopped: AtomicBool,
    active_count: AtomicUsize,
    notify: tokio::sync::Notify,
}

impl ContextTrackerInner {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            stopped: AtomicBool::new(false),
            active_count: AtomicUsize::new(0),
            notify: tokio::sync::Notify::new(),
        })
    }

    fn child(self: &Arc<Self>) -> ContextTracker {
        self.active_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        ContextTracker(self.clone())
    }

    fn stop(&self) {
        self.stopped
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    async fn wait(&self) {
        let notify = self.notify.notified();

        // If there are no active children, then the notify will never be called
        if self.active_count.load(std::sync::atomic::Ordering::Relaxed) == 0 {
            return;
        }

        notify.await;
    }
}

#[derive(Clone, Debug)]
pub struct Context {
    token: CancellationToken,
    _trackers: Vec<ContextTracker>,
}

impl Context {
    #[must_use]
    pub fn new() -> (Self, Handler) {
        Handler::global().new_child()
    }

    #[must_use]
    pub fn new_child(&self) -> (Self, Handler) {
        let token = self.token.child_token();
        let tracker = ContextTrackerInner::new();

        (
            Self {
                _trackers: {
                    let mut trackers = self._trackers.clone();
                    trackers.push(tracker.child());
                    trackers
                },
                token: token.clone(),
            },
            Handler {
                _token: TokenDropGuard(token),
                tracker,
            },
        )
    }

    #[must_use]
    pub fn global() -> Self {
        Handler::global().context()
    }

    pub async fn done(&self) {
        self.token.cancelled().await;
    }

    pub async fn into_done(self) {
        self.done().await;
    }

    #[must_use]
    pub fn is_done(&self) -> bool {
        self.token.is_cancelled()
    }
}

struct TokenDropGuard(CancellationToken);

impl TokenDropGuard {
    #[must_use]
    fn child(&self) -> CancellationToken {
        self.0.child_token()
    }

    fn cancel(&self) {
        self.0.cancel();
    }
}

impl Drop for TokenDropGuard {
    fn drop(&mut self) {
        self.cancel();
    }
}

pub struct Handler {
    _token: TokenDropGuard,
    tracker: Arc<ContextTrackerInner>,
}

impl Default for Handler {
    fn default() -> Self {
        Self::new()
    }
}

impl Handler {
    #[must_use]
    pub fn new() -> Handler {
        let token = CancellationToken::new();
        let tracker = ContextTrackerInner::new();

        Handler {
            _token: TokenDropGuard(token),
            tracker,
        }
    }

    #[must_use]
    pub fn global() -> &'static Self {
        static GLOBAL: once_cell::sync::Lazy<Handler> = once_cell::sync::Lazy::new(Handler::new);
        &GLOBAL
    }

    pub async fn shutdown(&self) {
        self.tracker.stop();
        self.cancel();
        self.tracker.wait().await;
    }

    #[must_use]
    pub fn context(&self) -> Context {
        Context {
            token: self._token.child(),
            _trackers: vec![self.tracker.child()],
        }
    }

    #[must_use]
    pub fn new_child(&self) -> (Context, Handler) {
        self.context().new_child()
    }

    pub fn cancel(&self) {
        self._token.cancel();
    }
}

pub trait ContextExt {
    fn context(self, ctx: Context) -> FutureWithContext<Self>
    where
        Self: Sized;
}

impl<F: Future> ContextExt for F {
    fn context(self, ctx: Context) -> FutureWithContext<Self> {
        FutureWithContext {
            future: self,
            _channels: ctx._trackers,
            ctx: Box::pin(ctx.token.cancelled_owned()),
        }
    }
}

#[pin_project::pin_project]
pub struct FutureWithContext<F> {
    #[pin]
    future: F,
    _channels: Vec<ContextTracker>,
    ctx: Pin<Box<WaitForCancellationFutureOwned>>,
}

impl<F: Future> Future for FutureWithContext<F> {
    type Output = Option<F::Output>;

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.as_mut().project();

        match (this.ctx.as_mut().poll(cx), this.future.poll(cx)) {
            (_, Poll::Ready(v)) => std::task::Poll::Ready(Some(v)),
            (Poll::Ready(_), Poll::Pending) => std::task::Poll::Ready(None),
            _ => std::task::Poll::Pending,
        }
    }
}
