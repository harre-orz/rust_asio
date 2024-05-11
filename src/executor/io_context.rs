use super::{AsyncSocket, Reactor};
use crate::error::OsError;
use crate::ffi::ConnectedSocket;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

struct Inner {
    reactor: Reactor,
    stop: AtomicBool,
}

struct FutureBlock(Arc<Inner>);

impl Future for FutureBlock {
    type Output = Result<(), OsError>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        self.0.reactor.poll(ctx)
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
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub fn restart(&self) -> bool {
        match self
            .inner
            .stop
            .compare_exchange(true, false, Ordering::Acquire, Ordering::Relaxed)
        {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub async fn run(&self) -> Result<bool, OsError> {
        if let Err(err) = FutureBlock(self.inner.clone()).await {
            Err(err)
        } else {
            Ok(self.stop())
        }
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
