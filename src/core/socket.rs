use super::IoContext;
use super::{Deadline, Event};
use super::{Socket, Timeout};
use crate::error::Result;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
#[cfg(windows)]
use windows_sys::Win32::System::IO;

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
        match self.event.lock().unwrap().read_poll(ctx) {
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
        match self.event.lock().unwrap().write_poll(ctx) {
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
        self.ctx.inner.reactor.del_socket(&self.soc)
    }
}

impl AsyncSocket {
    pub(crate) fn new(ctx: IoContext, soc: Socket) -> Self {
        let event: Event = Default::default();
        ctx.inner.reactor.add_socket(&soc, &event);
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

    // #[cfg(windows)]
    // pub(crate) fn iocp(&self, timeout: Timeout, ov: IO::OVERLAPPED) -> WaitForIocp {
    //     let timer = Deadline::new(timeout);
    //     self.wake();
    //     WaitForIocp {
    //         ctx: self.ctx.clone(),
    //         event: self.event.clone(),
    //         timer: timer,
    //         ov: ov,
    //     }
    // }
}
