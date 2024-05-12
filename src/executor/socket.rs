use super::{Event, IoContext};
use crate::error::OsError;
use crate::ffi::{ConnectedSocket, Timeout};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

pub(crate) struct WaitForReadable {
    event: Arc<Mutex<Event>>,
}

impl Future for WaitForReadable {
    type Output = Result<(), OsError>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        let mut event = self.event.lock().unwrap();
        event.read_poll(ctx)
    }
}

pub(crate) struct WaitForWritable {
    event: Arc<Mutex<Event>>,
}

impl Future for WaitForWritable {
    type Output = Result<(), OsError>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        let mut event = self.event.lock().unwrap();
        event.write_poll(ctx)
    }
}

pub(crate) struct AsyncSocket {
    ctx: IoContext,
    soc: ConnectedSocket,
    event: Arc<Mutex<Event>>,
}

impl Drop for AsyncSocket {
    fn drop(&mut self) {
        self.ctx.as_reactor().deregister_socket(&self.soc)
    }
}

impl AsyncSocket {
    pub fn new(ctx: IoContext, soc: ConnectedSocket) -> Self {
        let event = ctx.as_reactor().register_socket(&soc);
        Self { ctx, soc, event }
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn as_socket(&self) -> &ConnectedSocket {
        &self.soc
    }

    pub fn wait_for_readable(&self, timeout: Timeout) -> WaitForReadable {
        Event::read_reset(
            self.event.clone(),
            &self.ctx.as_reactor(),
            &self.soc,
            timeout,
        );
        WaitForReadable {
            event: self.event.clone(),
        }
    }

    pub fn wait_for_writable(&self, timeout: Timeout) -> WaitForWritable {
        Event::write_reset(
            self.event.clone(),
            &self.ctx.as_reactor(),
            &self.soc,
            timeout,
        );
        WaitForWritable {
            event: self.event.clone(),
        }
    }
}
