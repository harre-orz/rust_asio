use super::Reactor;
use crate::error::OsError;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

struct Inner {
    reactor: Reactor,
    stop: AtomicBool,
}

struct FutureRun(Arc<Inner>);

impl Future for FutureRun {
    type Output = Result<(), OsError>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        if self.0.stop.load(Ordering::SeqCst) {
            self.0.reactor.stop();
            Poll::Ready(Ok(()))
        } else {
            self.0.reactor.poll(ctx)
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
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub async fn run(&self) -> Result<bool, OsError> {
        if let Err(err) = FutureRun(self.inner.clone()).await {
            Err(err)
        } else {
            Ok(!self.inner.stop.swap(false, Ordering::SeqCst))
        }
    }

    pub(super) fn as_reactor(&self) -> &Reactor {
        &self.inner.reactor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run() {
        let ctx = IoContext::new().unwrap();
        assert_eq!(ctx.is_stopped(), false);
        assert_eq!(ctx.run().await, Ok(true));
        assert_eq!(ctx.is_stopped(), false);
    }

    #[tokio::test]
    async fn stop() {
        let ctx = IoContext::new().unwrap();
        assert_eq!(ctx.is_stopped(), false);
        assert_eq!(ctx.stop(), true);
        assert_eq!(ctx.is_stopped(), true);
        assert_eq!(ctx.run().await, Ok(false));
        assert_eq!(ctx.is_stopped(), false);
    }

    #[test]
    fn stop2() {
        let ctx = IoContext::new().unwrap();
        assert_eq!(ctx.is_stopped(), false);
        assert_eq!(ctx.stop(), true);
        assert_eq!(ctx.is_stopped(), true);
    }
}
