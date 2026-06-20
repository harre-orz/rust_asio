use crate::error::OsError;
#[cfg(target_os = "macos")]
use crate::primitive::Signal;
use crate::primitive::{AtomicTimeout, Socket, TimeoutError};
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
use self::poll::Reactor;
pub(crate) use self::poll::{Event, EventGuard};

mod scheduler;
use self::scheduler::Scheduler;

struct Inner {
    reactor: Reactor,
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
            return Poll::Ready(Ok(()));
        }

        if self.0.stop.load(Ordering::Relaxed) {
            self.0.reactor.cancel_all_events(&self.0.scheduler);
            Poll::Pending
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
    inner: Arc<Inner>,
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
                self.inner.reactor.wake_up_now();
                true
            }
            Err(_) => false,
        }
    }

    pub async fn run(&self) -> Result<(), OsError> {
        FutureRun(self.inner.clone()).await
    }

    pub(crate) fn new_socket<T>(
        &self,
        soc: Socket,
        ato: AtomicTimeout,
        data: T,
    ) -> Pin<Box<(Event, (IoContext, Socket, AtomicTimeout, T))>> {
        let ev = Event::new((self.clone(), soc, ato, data));
        self.inner.reactor.add_socket(&ev.1.1, &ev.as_ref().0);
        ev
    }

    pub(crate) fn del_socket(&self, soc: &Socket) {
        self.inner.reactor.del_socket(soc)
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn add_signal(&self, sig: Signal, ev: &Event) {
        self.inner.reactor.add_signal(sig, ev);
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn del_signal(&self, sig: Signal) {
        self.inner.reactor.del_signal(sig);
    }

    pub fn set_timeout(&self, timer: Duration) -> Result<(), TimeoutError> {
        let millis = timer.as_millis();
        if millis > i32::MAX as u128 {
            self.inner
                .default_timeout
                .store(millis as i32, Ordering::SeqCst);
            Ok(())
        } else {
            Err(TimeoutError)
        }
    }

    pub(crate) fn timeout(&self) -> AtomicTimeout {
        AtomicTimeout::new(&self.inner.default_timeout)
    }

    pub(crate) fn lock<'a, 'b>(&'a self, event: &'b Event) -> EventGuard<'a, 'b> {
        if let Some(waker) = self.inner.waker.lock().unwrap().take() {
            waker.wake();
        }
        EventGuard::lock(&self.inner, &event)
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
