use super::{AsyncSocket, Reactor};
use crate::error::OsError;
use crate::ffi::ConnectedSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::pin::Pin;
use std::future::Future;


struct Inner {
    reactor: Reactor,
    stop: AtomicBool,
}

pub struct FutureBlock {
    inner: Arc<Inner>,
}

impl Future for FutureBlock {
    type Output = Result<usize, OsError>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        self.inner.reactor.poll(ctx)
    }
}

#[derive(Clone)]
pub struct IoContext {
    inner: Arc<Inner>,
}

impl IoContext {
    pub fn new() -> Result<Self, OsError> {
        Ok(Self {
            inner: Arc::new(
                Inner {
                    reactor: Reactor::new()?,
                    stop: AtomicBool::new(false),
                }
            )
        })
    }

    pub fn is_stopped(&self) -> bool {
        self.inner.stop.load(Ordering::SeqCst)
    }

    pub fn stop(&self) {
        self.inner.stop.store(true, Ordering::SeqCst)
    }

    pub async fn run(&self) -> Result<usize, OsError> {
        let mut count = 0;
        while !self.is_stopped() {
            let block = FutureBlock { inner: self.inner.clone() };
            match block.await {
                Ok(len) => count += len,
                Err(_) if count != 0 => return Ok(count),
                Err(err) => return Err(err),
            }
        }
        Ok(count)
    }

    pub(crate) fn async_socket(&self, soc: ConnectedSocket) -> AsyncSocket {
        self.inner.reactor.register_socket(soc, self)
    }

    pub(super) fn drop_socket(&self, soc: &ConnectedSocket) {
        self.inner.reactor.deregister_socket(soc)
    }

    pub(super) fn as_reactor(&self) -> &Reactor {
        &self.inner.reactor
    }
}
