use ffi::{AsRawFd, RawFd, SystemError, Timeout, socket, bind, listen, ioctl, getsockopt,
          setsockopt, getsockname};
use reactor::{SocketImpl};
use core::{Protocol, Socket, IoControl, GetSocketOption, SetSocketOption, AsIoContext,
           IoContext, Perform, ThreadIoContext, Cancel};
use handler::{Handler, AsyncReadOp};
use socket_base::MAX_CONNECTIONS;

use std::io;
use std::fmt;
use std::time::Duration;

use accept_ops::{async_accept, blocking_accept, nonblocking_accept};

pub struct SocketListener<P> {
    pimpl: Box<SocketImpl<P>>,
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
        Ok(blocking_accept(self, &self.pimpl.timeout)?)
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
        self.pimpl.cancel()
    }

    pub fn listen(&self) -> io::Result<()> {
        Ok(listen(self, MAX_CONNECTIONS)?)
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        Ok(getsockname(self)?)
    }

    pub fn nonblicking_accept(&self) -> io::Result<(P::Socket, P::Endpoint)> {
        Ok(nonblocking_accept(self)?)
    }

    pub fn get_timeout(&self) -> Duration {
        self.pimpl.timeout.get()
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

    pub fn set_timeout(&self, timeout: Duration) -> io::Result<()> {
        Ok(self.pimpl.timeout.set(timeout)?)
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
        self.pimpl.as_ctx()
    }
}

impl<P> AsRawFd for SocketListener<P> {
    fn as_raw_fd(&self) -> RawFd {
        self.pimpl.as_raw_fd()
    }
}

impl<P> Cancel for SocketListener<P> {
    fn cancel(&self) {
        self.pimpl.cancel()
    }
}

impl<P> AsyncReadOp for SocketListener<P>
where
    P: Protocol,
{
    fn add_read_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.pimpl.add_read_op(this, op, err)
    }

    fn next_read_op(&self, this: &mut ThreadIoContext) {
        self.pimpl.next_read_op(this)
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
        &self.pimpl.data
    }

    unsafe fn from_raw_fd(ctx: &IoContext, soc: RawFd, pro: P) -> Self {
        SocketListener { pimpl: SocketImpl::new(ctx, soc, pro) }
    }
}
