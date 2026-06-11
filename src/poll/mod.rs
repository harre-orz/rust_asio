use crate::error::Result;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Timeout(pub(crate) libc::c_int);

impl Timeout {
    pub const fn infinite() -> Self {
        Self(-1)
    }

    pub const fn from_duration(timeout: Duration) -> Self {
        let time = timeout.as_millis();
        if time > i32::MAX as u128 {
            Timeout::infinite()
        } else {
            Timeout(time as i32)
        }
    }

    pub const fn into_duration(self) -> Duration {
        let millis = if self.0 == -1 {
            u32::MAX
        } else {
            self.0 as u32
        };
        Duration::from_millis(millis as u64)
    }
}

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use self::unix::{Fd, Signal};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use self::windows::{AsRawHandle, Handle};

#[cfg(all(feature = "timerfd", any(target_os = "linux")))]
mod intr_timerfd;
#[cfg(all(feature = "timerfd", any(target_os = "linux")))]
use self::intr_timerfd::TimerFd as Intr;

#[cfg(all(feature = "eventfd", any(target_os = "linux")))]
mod intr_eventfd;
#[cfg(all(feature = "eventfd", any(target_os = "linux")))]
use self::intr_eventfd::EventFd as Intr;

#[cfg(any(
    target_os = "macos",
    not(any(windows, feature = "timerfd", feature = "eventfd"))
))]
mod intr_pipe_unix;
#[cfg(any(
    target_os = "macos",
    not(any(windows, feature = "timerfd", feature = "eventfd"))
))]
pub(crate) use self::intr_pipe_unix::Pipe as Intr;

#[cfg(windows)]
mod intr_pipe_win;
#[cfg(windows)]
use self::intr_pipe_win::Pipe as Intr;

#[cfg(target_os = "linux")]
mod poll_epoll;
#[cfg(target_os = "linux")]
pub(crate) use self::poll_epoll::{Epoll as Reactor, Event};

#[cfg(target_os = "macos")]
mod poll_kqueue;
#[cfg(target_os = "macos")]
pub(crate) use self::poll_kqueue::{Event, Kqueue as Reactor};

#[cfg(windows)]
mod poll_iocp;
#[cfg(windows)]
pub(crate) use self::poll_iocp::{Event, Iocp as Reactor};

mod scheduler;
pub(crate) use self::scheduler::{Deadline, EventScheduler};

pub(crate) struct Inner {
    pub(crate) reactor: Reactor,
    pub(crate) scheduler: EventScheduler,
    pub(crate) waker: Mutex<Option<Waker>>,
    stop: AtomicBool,
}

struct FutureRun(Arc<Inner>);

impl Future for FutureRun {
    type Output = crate::error::Result<()>;

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
    pub(crate) inner: Arc<Inner>,
}

impl IoContext {
    pub fn new() -> crate::error::Result<Self> {
        let reactor = Reactor::new()?;
        Ok(Self {
            inner: Arc::new(Inner {
                waker: Mutex::new(None),
                reactor: reactor,
                scheduler: EventScheduler::new(),
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
                self.inner.reactor.intr.wake_up_now();
                true
            }
            Err(_) => false,
        }
    }

    pub async fn run(&self) -> Result<()> {
        FutureRun(self.inner.clone()).await
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
