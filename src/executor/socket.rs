use super::{Event, IoContext};
use crate::error::OsError;
use crate::ffi::{Socket, Timeout};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub(crate) struct WaitForReadable {
    event: Event,
}

impl Future for WaitForReadable {
    type Output = Result<(), OsError>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        self.event.read_poll(ctx)
    }
}

pub(crate) struct WaitForWritable {
    event: Event,
}

impl Future for WaitForWritable {
    type Output = Result<(), OsError>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        self.event.write_poll(ctx)
    }
}

pub(crate) struct AsyncSocket {
    ctx: IoContext,
    soc: Socket,
    event: Event,
}

impl Drop for AsyncSocket {
    fn drop(&mut self) {
        self.ctx.as_reactor().deregister_socket(&self.soc)
    }
}

impl AsyncSocket {
    pub fn new(ctx: IoContext, soc: Socket) -> Self {
        let event = ctx.as_reactor().register_socket(&soc);
        Self { ctx, soc, event }
    }

    pub fn as_socket(&self) -> &Socket {
        &self.soc
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn wait_for_readable(&self, timeout: Timeout) -> WaitForReadable {
        Event::read_reset(self.event.clone(), &self.ctx.as_reactor(), timeout);
        WaitForReadable {
            event: self.event.clone(),
        }
    }

    pub fn wait_for_writable(&self, timeout: Timeout) -> WaitForWritable {
        Event::write_reset(self.event.clone(), &self.ctx.as_reactor(), timeout);
        WaitForWritable {
            event: self.event.clone(),
        }
    }
}
