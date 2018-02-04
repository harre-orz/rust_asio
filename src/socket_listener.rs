#![allow(unreachable_patterns)]

use prelude::*;
use ffi::*;
use core::{AsIoContext, InnerSocket, IoContext, Perform, ThreadIoContext};
use handler::Handler;
use ops::{accept_timeout, async_accept, nonblocking_accept, AsyncReadOp};
use socket_base;

use std::io;
use std::fmt;

pub struct SocketListener<P> {
    inner: Box<InnerSocket<P>>,
}

impl<P> SocketListener<P>
where
    P: Protocol,
{
    pub fn new(ctx: &IoContext, pro: P) -> io::Result<Self> {
        let soc = socket(&pro)?;
        Ok(unsafe { Self::from_raw_fd(ctx, soc, pro) })
    }

    pub fn accept(&self) -> io::Result<(P::Socket, P::Endpoint)> {
        Ok(accept_timeout(self, &Timeout::default())?)
    }

    pub fn async_accept<F>(&self, handler: F) -> F::Output
    where
        F: Handler<(P::Socket, P::Endpoint), io::Error>,
    {
        async_accept(self, handler)
    }

    pub fn bind(&self, ep: &P::Endpoint) -> io::Result<()> {
        Ok(bind(self, ep)?)
    }

    pub fn cancel(&self) {
        self.inner.cancel()
    }

    pub fn listen(&self) -> io::Result<()> {
        Ok(listen(self, socket_base::MAX_CONNECTIONS)?)
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        Ok(getsockname(self)?)
    }

    pub fn nonblicking_accept(&self) -> io::Result<(P::Socket, P::Endpoint)> {
        Ok(nonblocking_accept(self)?)
    }

    pub fn get_option<C>(&self) -> io::Result<C>
    where
        C: GetSocketOption<P>,
    {
        Ok(getsockopt(self)?)
    }

    pub fn io_control<C>(&self, cmd: &mut C) -> io::Result<()>
    where
        C: IoControl,
    {
        Ok(ioctl(self, cmd)?)
    }

    pub fn set_option<C>(&self, cmd: C) -> io::Result<()>
    where
        C: SetSocketOption<P>,
    {
        Ok(setsockopt(self, cmd)?)
    }
}

unsafe impl<P> AsIoContext for SocketListener<P> {
    fn as_ctx(&self) -> &IoContext {
        self.inner.as_ctx()
    }
}

impl<P> AsRawFd for SocketListener<P> {
    fn as_raw_fd(&self) -> RawFd {
        self.inner.as_raw_fd()
    }
}

impl<P> AsyncReadOp for SocketListener<P>
where
    P: Protocol,
{
    fn add_read_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.inner.add_read_op(this, op, err)
    }

    fn next_read_op(&self, this: &mut ThreadIoContext) {
        self.inner.next_read_op(this)
    }
}

impl<P> fmt::Debug for SocketListener<P>
where
    P: Protocol + fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}({})", self.protocol(), self.as_raw_fd())
    }
}

unsafe impl<P> Send for SocketListener<P> {}

unsafe impl<P> Sync for SocketListener<P> {}

impl<P> Socket<P> for SocketListener<P>
where
    P: Protocol,
{
    fn protocol(&self) -> &P {
        &self.inner.data
    }

    unsafe fn from_raw_fd(ctx: &IoContext, soc: RawFd, pro: P) -> Self {
        SocketListener {
            inner: InnerSocket::new(ctx, soc, pro),
        }
    }
}
