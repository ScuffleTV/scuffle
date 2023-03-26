use std::time::Duration;

use futures::Future;
use tokio::time::Timeout;

pub trait FutureTimeout: Future {
    #[inline(always)]
    fn timeout(self, duration: Duration) -> Timeout<Self>
    where
        Self: Sized,
    {
        tokio::time::timeout(duration, self)
    }
}

impl<F: Future> FutureTimeout for F {}
