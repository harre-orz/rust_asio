use super::{Event, IoContext};
use crate::error::OsError;
use crate::socket::ffi::Socket;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Instant;

pub struct WaitForReadable(IoContext, Arc<Mutex<Event>>);

impl Future for WaitForReadable {
    type Output = Result<(), OsError>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        let mut event = self.1.lock().unwrap();
        event.read_poll(ctx, self.0.as_counter())
    }
}

pub struct WaitForWritable(IoContext, Arc<Mutex<Event>>);

impl Future for WaitForWritable {
    type Output = Result<(), OsError>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        let mut event = self.1.lock().unwrap();
        event.write_poll(ctx, self.0.as_counter())
    }
}

pub struct AsyncSocket {
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
    pub fn new(ctx: IoContext, soc: Socket) -> Self {
        let event = ctx.as_reactor().register_socket(&soc);
        Self {
            ctx: ctx,
            soc: soc,
            event: event,
        }
    }

    pub const fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub const fn as_socket(&self) -> &Socket {
        &self.soc
    }

    pub fn update_schedule(&self, cto: Instant) {
        self.ctx.as_reactor().update_schedule(&self.event, cto)
    }

    pub fn wait_for_readable(&self) -> WaitForReadable {
        self.ctx.as_reactor().ready_poll();
        WaitForReadable(self.ctx.clone(), self.event.clone())
    }

    pub fn wait_for_writable(&self) -> WaitForWritable {
        self.ctx.as_reactor().ready_poll();
        WaitForWritable(self.ctx.clone(), self.event.clone())
    }
}
