use super::{Deadline, Event, EventScheduler, Reactor};
use crate::error::Result;
use crate::socket::{Socket, Timeout};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use windows_sys::Win32::System::IO;

struct Inner {
    waker: Mutex<Option<Waker>>,
    reactor: Reactor,
    scheduler: EventScheduler,
    stop: AtomicBool,
    #[cfg(windows)]
    winsock: crate::socket::WinSockEx,
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
        #[cfg(windows)]
        let winsock = crate::socket::WinSockEx::new()?;

        let reactor = Reactor::new()?;
        Ok(Self {
            inner: Arc::new(Inner {
                waker: Mutex::new(None),
                reactor: reactor,
                scheduler: EventScheduler::new(),
                stop: AtomicBool::new(false),
                #[cfg(windows)]
                winsock: winsock,
            }),
        })
    }

    #[cfg(windows)]
    pub fn winsock(&self) -> &crate::socket::WinSockEx {
        &self.inner.winsock
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

#[cfg(unix)]
pub(crate) struct WaitForReadable {
    ctx: IoContext,
    event: Event,
    timer: Deadline,
}

#[cfg(unix)]
impl Future for WaitForReadable {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        match self.event.read_poll(ctx) {
            Poll::Pending => {
                if self
                    .ctx
                    .inner
                    .scheduler
                    .insert_event(&self.event, self.timer)
                {
                    self.ctx.inner.reactor.intr.wake_up_alarm(self.timer)
                }
                Poll::Pending
            }
            Poll::Ready(res) => Poll::Ready(res),
        }
    }
}

#[cfg(unix)]
pub(crate) struct WaitForWritable {
    ctx: IoContext,
    event: Event,
    timer: Deadline,
}

#[cfg(unix)]
impl Future for WaitForWritable {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        match self.event.write_poll(ctx) {
            Poll::Pending => {
                if self
                    .ctx
                    .inner
                    .scheduler
                    .insert_event(&self.event, self.timer)
                {
                    let timer = self.timer;
                    self.ctx.inner.reactor.intr.wake_up_alarm(timer)
                }
                Poll::Pending
            }
            Poll::Ready(res) => Poll::Ready(res),
        }
    }
}

#[cfg(windows)]
pub struct WaitForIocp {
    ctx: IoContext,
    event: Event,
    timer: Deadline,
    ov: IO::OVERLAPPED,
}

#[cfg(windows)]
impl Future for WaitForIocp {
    type Output = Result<usize>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        self.event.poll(ctx)
    }
}

pub(crate) struct AsyncSocket {
    ctx: IoContext,
    soc: Socket,
    event: Event,
}

impl Drop for AsyncSocket {
    fn drop(&mut self) {
        self.ctx.inner.reactor.deregister_soc(&self.soc)
    }
}

impl AsyncSocket {
    pub(crate) fn new(ctx: IoContext, soc: Socket) -> Self {
        let event = Event::new();
        ctx.inner.reactor.register_soc(&soc, &event);
        Self {
            ctx: ctx,
            soc: soc,
            event: event,
        }
    }

    pub(crate) const fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub(crate) const fn as_socket(&self) -> &Socket {
        &self.soc
    }

    fn wake(&self) {
        if let Some(waker) = self.ctx.inner.waker.lock().unwrap().take() {
            waker.wake();
        }
    }

    #[cfg(unix)]
    pub(crate) fn poll_in(&self, timeout: Timeout) -> WaitForReadable {
        let timer = Deadline::new(timeout);
        self.wake();
        WaitForReadable {
            ctx: self.ctx.clone(),
            event: self.event.clone(),
            timer: timer,
        }
    }

    #[cfg(unix)]
    pub(crate) fn poll_out(&self, timeout: Timeout) -> WaitForWritable {
        let timer = Deadline::new(timeout);
        self.wake();
        WaitForWritable {
            ctx: self.ctx.clone(),
            event: self.event.clone(),
            timer: timer,
        }
    }

    pub(crate) fn iocp(&self, timeout: Timeout, ov: IO::OVERLAPPED) -> WaitForIocp {
        let timer = Deadline::new(timeout);
        self.wake();
        WaitForIocp {
            ctx: self.ctx.clone(),
            event: self.event.clone(),
            timer: timer,
            ov: ov,
        }
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
