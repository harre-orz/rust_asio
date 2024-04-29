use super::{Event, IoContext};
use crate::error::OsError;
use crate::ffi::ConnectedSocket;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

pub struct WaitForReadable {
    event: Arc<Mutex<Event>>,
}

// impl WaitForReadable {
//     fn new(soc: &AsyncSocket, timeout: Duration, reactor: &Arc<Reactor>) -> Self {
//         //let mut event = soc.event.lock().unwrap();
//         Event::read_reset(soc.event, timeout)
//         Self {
//             event: soc.event.clone()
//         }
//     }
// }

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

// impl WaitForWritable {
//     fn new(soc: &AsyncSocket, timeout: Duration) -> Self {
//         //let mut event = soc.event.lock().unwrap();
//         Self {
//             event: soc.event.clone()
//         }
//     }
// }

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
        F: FnOnce(&mut Event),
    {
        let event = unsafe { &*(ptr as *const Arc<Mutex<Event>>) };
        let mut event = event.lock().unwrap();
        func(&mut event)
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn as_socket(&self) -> &ConnectedSocket {
        &self.soc
    }

    pub fn wait_for_readable(&self, timeout: Duration) -> WaitForReadable {
        Event::read_reset(&self.event, &self.soc, timeout, &self.ctx.as_reactor());
        WaitForReadable {
            event: self.event.clone(),
        }
    }

    pub fn wait_for_writable(&self, timeout: Duration) -> WaitForWritable {
        Event::write_reset(&self.event, &self.soc, timeout, &self.ctx.as_reactor());
        WaitForWritable {
            event: self.event.clone(),
        }
    }
}
