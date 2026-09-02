use crate::error::OsError;
#[cfg(target_os = "macos")]
use crate::primitive::Signal;
use crate::primitive::{AtomicTimeout, DurationOverflowError};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

mod deadline;
use self::deadline::Deadline;

mod intr;
use self::intr::Intr;

mod poll;
pub(crate) use self::poll::{Event, EventGuard, Reactor};

mod scheduler;
use self::scheduler::Scheduler;

pub(crate) struct Inner {
    pub(crate) reactor: Reactor,
    scheduler: Scheduler,
    waker: Mutex<Option<Waker>>,
    stop: AtomicBool,
    default_timeout: AtomicI32,
}

struct FutureRun(Arc<Inner>);

impl Future for FutureRun {
    type Output = Result<(), OsError>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        if self.0.scheduler.pending_count() == 0 {
            Poll::Ready(Ok(()))
        } else {
            match self.0.reactor.poll(&self.0.scheduler) {
                Poll::Pending => {
                    let mut waker = self.0.waker.lock().unwrap();
                    *waker = Some(ctx.waker().clone());
                    Poll::Pending
                }
                Poll::Ready(err) => Poll::Ready(Err(err)),
            }
        }
    }
}

#[derive(Clone)]
pub struct IoContext {
    pub(crate) inner: Arc<Inner>,
}

impl IoContext {
    pub fn new() -> Result<Self, OsError> {
        let reactor = Reactor::new()?;
        Ok(Self {
            inner: Arc::new(Inner {
                waker: Mutex::new(None),
                reactor: reactor,
                scheduler: Scheduler::new(),
                stop: AtomicBool::new(false),
                default_timeout: AtomicI32::new(i32::MAX),
            }),
        })
    }

    fn wake_up(&self) {
        if let Some(waker) = self.inner.waker.lock().unwrap().take() {
            waker.wake();
        }
    }

    pub(crate) fn lock<'a, 'b>(&'a self, event: &'b Event) -> EventGuard<'a, 'b> {
        self.wake_up();
        EventGuard::lock(&self.inner, &event)
    }

    pub fn stop(&self) -> bool {
        match self
            .inner
            .stop
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        {
            Ok(_) => {
                self.inner.reactor.cancel_all_events(&self.inner.scheduler);
                self.wake_up();
                true
            }
            Err(_) => false,
        }
    }

    pub fn is_stopped(&self) -> bool {
        self.inner.stop.load(Ordering::SeqCst)
    }

    pub async fn run(&self) -> Result<(), OsError> {
        FutureRun(self.inner.clone()).await
    }

    pub fn default_timeout(&self, timeout: Duration) -> Result<(), DurationOverflowError> {
        let millis = timeout.as_millis();
        if millis > i32::MAX as u128 {
            self.inner
                .default_timeout
                .store(millis as i32, Ordering::SeqCst);
            Ok(())
        } else {
            Err(DurationOverflowError)
        }
    }

    pub(crate) fn timeout(&self) -> AtomicTimeout {
        AtomicTimeout::new(&self.inner.default_timeout)
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
