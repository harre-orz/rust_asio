use super::{Event, IoContext};
use crate::error::OsError;
use crate::ffi::{ConnectedSocket, Timeout};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

pub struct WaitForReadable {
    event: Arc<Mutex<Event>>,
}

impl Future for WaitForReadable {
    type Output = Result<(), OsError>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        let mut event = self.event.lock().unwrap();
        event.read_poll(ctx)
    }
}

pub struct WaitForWritable {
    event: Arc<Mutex<Event>>,
}

impl Future for WaitForWritable {
    type Output = Result<(), OsError>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        let mut event = self.event.lock().unwrap();
        event.write_poll(ctx)
    }
}

pub struct AsyncSocket {
    ctx: IoContext,
    soc: ConnectedSocket,
    event: Arc<Mutex<Event>>,
}

impl Drop for AsyncSocket {
    fn drop(&mut self) {
        self.ctx.drop_socket(&self.soc)
    }
}

impl AsyncSocket {
    pub(super) fn new(ctx: &IoContext, soc: ConnectedSocket) -> Self {
        Self {
            ctx: ctx.clone(),
            soc: soc,
            event: Arc::new(Mutex::new(Event::new())),
        }
    }

    pub(super) fn as_epoll_ptr(&self) -> u64 {
        Arc::as_ptr(&self.event) as u64
    }

    pub(super) fn epoll_op<F>(ptr: u64, func: F)
    where
        F: FnOnce(&mut Event) -> Option<Waker>,
    {
        if let Some(waker) = {
            let event = unsafe { Arc::from_raw(ptr as *const Mutex<Event>) };
            let mut op = event.lock().unwrap();
            func(&mut op)
        } {
            waker.wake()
        }
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn as_socket(&self) -> &ConnectedSocket {
        &self.soc
    }

    pub fn wait_for_readable(&self, timeout: Timeout) -> WaitForReadable {
        Event::read_reset(&self.event, &self.soc, timeout, &self.ctx.as_reactor());
        WaitForReadable {
            event: self.event.clone(),
        }
    }

    pub fn wait_for_writable(&self, timeout: Timeout) -> WaitForWritable {
        Event::write_reset(&self.event, &self.soc, timeout, &self.ctx.as_reactor());
        WaitForWritable {
            event: self.event.clone(),
        }
    }
}
