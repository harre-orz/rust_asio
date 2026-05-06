use super::{Event, Reactor};
use crate::error::OsError;
use crate::socket::Socket;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Instant;

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

pub(crate) struct WaitForReadable {
    ctx: IoContext,
    event: Arc<Mutex<Event>>,
}

impl Future for WaitForReadable {
    type Output = Result<(), OsError>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        let mut event = self.event.lock().unwrap();
        event.read_poll(ctx, self.ctx.as_counter())
    }
}

pub(crate) struct WaitForWritable {
    ctx: IoContext,
    event: Arc<Mutex<Event>>,
}

impl Future for WaitForWritable {
    type Output = Result<(), OsError>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        let mut event = self.event.lock().unwrap();
        event.write_poll(ctx, self.ctx.as_counter())
    }
}

pub(crate) struct AsyncSocket {
    ctx: IoContext,
    soc: Socket,
    event: Arc<Mutex<Event>>,
}

impl Drop for AsyncSocket {
    fn drop(&mut self) {
        self.ctx
            .as_reactor()
            .deregister_socket(&self.soc, &self.event)
    }
}

impl AsyncSocket {
    pub(crate) fn new(ctx: IoContext, soc: Socket) -> Self {
        let event = ctx.as_reactor().register_socket(&soc);
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

    pub(crate) fn update_schedule(&self, cto: Instant) {
        self.ctx.as_reactor().update_schedule(&self.event, cto)
    }

    pub(crate) fn wait_for_readable(&self) -> WaitForReadable {
        self.ctx.as_reactor().ready_poll();
        WaitForReadable {
            ctx: self.ctx.clone(),
            event: self.event.clone(),
        }
    }

    pub(crate) fn wait_for_writable(&self) -> WaitForWritable {
        self.ctx.as_reactor().ready_poll();
        WaitForWritable {
            ctx: self.ctx.clone(),
            event: self.event.clone(),
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
