use super::reactor::Reactor;
use crate::error::OsError;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::task::{Context, Poll};

struct Inner {
    reactor: Reactor,
    stop: AtomicBool,
    count: AtomicUsize,
}

struct FutureRun(Arc<Inner>);

impl Future for FutureRun {
    type Output = Result<(), OsError>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        if self.0.count.load(Ordering::Relaxed) > 0 {
            self.0.reactor.poll(ctx)
        } else {
            return Poll::Ready(Ok(()));
        }
    }
}

#[derive(Clone)]
pub struct IoContext {
    inner: Arc<Inner>,
}

impl IoContext {
    pub fn new() -> Result<Self, OsError> {
        Ok(Self {
            inner: Arc::new(Inner {
                reactor: Reactor::new()?,
                stop: AtomicBool::new(false),
                count: AtomicUsize::new(0),
            }),
        })
    }

    pub fn is_stopped(&self) -> bool {
        self.inner.stop.load(Ordering::SeqCst)
    }

    pub fn stop(&self) -> bool {
        match self
            .inner
            .stop
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        {
            Ok(_) => {
                self.inner.reactor.stop_request();
                true
            }
            Err(_) => false,
        }
    }

    pub async fn run(&self) -> Result<(), OsError> {
        FutureRun(self.inner.clone()).await
    }

    pub(super) fn as_reactor(&self) -> &Reactor {
        &self.inner.reactor
    }

    pub(super) fn as_counter(&self) -> &AtomicUsize {
        &self.inner.count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run() {
        let ctx = IoContext::new().unwrap();
        assert_eq!(ctx.is_stopped(), false);
        assert_eq!(ctx.run().await, Ok(()));
        assert_eq!(ctx.is_stopped(), false);
    }

    #[tokio::test]
    async fn stop() {
        let ctx = IoContext::new().unwrap();
        assert_eq!(ctx.is_stopped(), false);
        assert_eq!(ctx.stop(), true);
        assert_eq!(ctx.is_stopped(), true);
        assert_eq!(ctx.run().await, Ok(()));
        //assert_eq!(ctx.is_stopped(), false);
    }

    #[test]
    fn stop2() {
        let ctx = IoContext::new().unwrap();
        assert_eq!(ctx.is_stopped(), false);
        assert_eq!(ctx.stop(), true);
        assert_eq!(ctx.is_stopped(), true);
    }
}
