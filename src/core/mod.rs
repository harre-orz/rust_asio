use crate::error::Result;
use crate::primitive::{Signal, Socket};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

mod intr;
use self::intr::Intr;

mod poll;
pub(crate) use self::poll::AsyncEvent;
use self::poll::Reactor;

mod scheduler;
use self::scheduler::Scheduler;

struct Inner {
    reactor: Reactor,
    scheduler: Scheduler,
    waker: Mutex<Option<Waker>>,
    stop: AtomicBool,
}

struct FutureRun(Arc<Inner>);

impl Future for FutureRun {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        if self.0.scheduler.pending_count() == 0 {
            return Poll::Ready(Ok(()));
        }

        if self.0.stop.load(Ordering::Relaxed) {
            let mut vec = Vec::new();
            self.0.scheduler.cancel_all_events(&mut vec);
            for waker in vec {
                waker.wake();
            }
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
    pub fn new() -> Result<Self> {
        let reactor = Reactor::new()?;
        Ok(Self {
            inner: Arc::new(Inner {
                waker: Mutex::new(None),
                reactor: reactor,
                scheduler: Scheduler::new(),
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
            Ok(_) => {
                self.inner.reactor.wake_up_now();
                true
            }
            Err(_) => false,
        }
    }

    pub async fn run(&self) -> Result<()> {
        FutureRun(self.inner.clone()).await
    }

    pub(crate) fn wake(&self) {
        if let Some(waker) = self.inner.waker.lock().unwrap().take() {
            waker.wake();
        }
    }

    pub(crate) fn add_socket<T>(&self, soc: Socket, data: T) -> AsyncEvent<(IoContext, Socket, T)> {
        let ev = AsyncEvent::new((self.clone(), soc, data));
        self.inner.reactor.add_socket(&ev.as_data().1, &ev);
        ev
    }

    pub(crate) fn del_socket<T>(&self, ev: &AsyncEvent<(IoContext, Socket, T)>) {
        self.inner.reactor.del_socket(&ev.as_data().1)
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn add_signal<T>(&self, sig: Signal, ev: &AsyncEvent<T>) {
        self.inner.reactor.add_signal(sig, ev);
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn del_signal<T>(&self, sig: Signal) {
        self.inner.reactor.del_signal(sig);
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
