use ffi::{AsRawFd, RawFd, SystemError, Timeout, socket, shutdown, bind, ioctl, getsockopt,
          setsockopt, getpeername, getsockname};
use core::{Protocol, Socket, IoControl, GetSocketOption, SetSocketOption, AsIoContext, SocketImpl,
           IoContext, Perform, ThreadIoContext, Cancel};
use handler::{Handler, AsyncReadOp, AsyncWriteOp};
use connect_ops::{async_connect, nonblocking_connect};
use read_ops::{Recv, RecvFrom, async_read_op, blocking_read_op, nonblocking_read_op};
use write_ops::{Sent, SendTo, async_write_op, blocking_write_op, nonblocking_write_op};
use socket_base::{BytesReadable, Shutdown};

use std::io;
use std::fmt;
use std::time::Duration;

pub struct DgramSocket<P> {
    pimpl: Box<SocketImpl<P>>,
}

impl<P> DgramSocket<P>
where
    P: Protocol,
{
    pub fn new(ctx: &IoContext, pro: P) -> io::Result<Self> {
        let soc = socket(&pro)?;
        Ok(unsafe { Self::from_raw_fd(ctx, soc, pro) })
    }

    pub fn async_connect<F>(&self, ep: &P::Endpoint, handler: F) -> F::Output
    where
        F: Handler<(), io::Error>,
    {
        async_connect(self, ep, handler)
    }

    pub fn async_receive<F>(&self, buf: &mut [u8], flags: i32, handler: F) -> F::Output
    where
        F: Handler<usize, io::Error>,
    {
        async_read_op(self, buf, handler, Recv::new(flags))
    }

    pub fn async_receive_from<F>(&self, buf: &mut [u8], flags: i32, handler: F) -> F::Output
    where
        F: Handler<(usize, P::Endpoint), io::Error>,
    {
        async_read_op(self, buf, handler, RecvFrom::new(flags))
    }

    pub fn async_send<F>(&self, buf: &[u8], flags: i32, handler: F) -> F::Output
    where
        F: Handler<usize, io::Error>,
    {
        async_write_op(self, buf, handler, Sent::new(flags))
    }

    pub fn async_send_to<F>(
        &self,
        buf: &[u8],
        flags: i32,
        ep: &P::Endpoint,
        handler: F,
    ) -> F::Output
    where
        F: Handler<usize, io::Error>,
    {
        async_write_op(self, buf, handler, SendTo::new(flags, ep))
    }

    pub fn available(&self) -> io::Result<usize> {
        let mut bytes = BytesReadable::default();
        ioctl(self, &mut bytes)?;
        Ok(bytes.get())
    }

    pub fn bind(&self, ep: &P::Endpoint) -> io::Result<()> {
        Ok(bind(self, ep)?)
    }

    pub fn connect(&self, ep: &P::Endpoint) -> io::Result<()> {
        nonblocking_connect(self, ep)
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        Ok(getsockname(self)?)
    }

    pub fn get_option<C>(&self) -> io::Result<C>
    where
        C: GetSocketOption<P>,
    {
        Ok(getsockopt(self)?)
    }

    pub fn get_timeout(&self) -> Duration {
        self.pimpl.timeout.get()
    }

    pub fn io_control<C>(&self, cmd: &mut C) -> io::Result<()>
    where
        C: IoControl,
    {
        Ok(ioctl(self, cmd)?)
    }

    pub fn nonblocking_receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        nonblocking_read_op(self, buf, Recv::new(flags))
    }

    pub fn nonblocking_receive_from(
        &self,
        buf: &mut [u8],
        flags: i32,
    ) -> io::Result<(usize, P::Endpoint)> {
        nonblocking_read_op(self, buf, RecvFrom::new(flags))
    }

    pub fn nonblocking_send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        nonblocking_write_op(self, buf, Sent::new(flags))
    }

    pub fn nonblocking_send_to(
        &self,
        buf: &[u8],
        flags: i32,
        ep: &P::Endpoint,
    ) -> io::Result<usize> {
        nonblocking_write_op(self, buf, SendTo::new(flags, ep))
    }

    pub fn receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        blocking_read_op(self, buf, &self.pimpl.timeout, Recv::new(flags))
    }

    pub fn receive_from(&self, buf: &mut [u8], flags: i32) -> io::Result<(usize, P::Endpoint)> {
        blocking_read_op(self, buf, &self.pimpl.timeout, RecvFrom::new(flags))
    }

    pub fn remote_endpoint(&self) -> io::Result<P::Endpoint> {
        Ok(getpeername(self)?)
    }

    pub fn send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        blocking_write_op(self, buf, &self.pimpl.timeout, Sent::new(flags))
    }

    pub fn send_to(&self, buf: &[u8], flags: i32, ep: &P::Endpoint) -> io::Result<usize> {
        blocking_write_op(self, buf, &self.pimpl.timeout, SendTo::new(flags, ep))
    }

    pub fn set_option<C>(&self, cmd: C) -> io::Result<()>
    where
        C: SetSocketOption<P>,
    {
        Ok(setsockopt(self, cmd)?)
    }

    pub fn set_timeout(&self, timeout: Duration) -> io::Result<()> {
        Ok(self.pimpl.timeout.set(timeout)?)
    }

    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        Ok(shutdown(self, how)?)
    }
}

unsafe impl<P> AsIoContext for DgramSocket<P> {
    fn as_ctx(&self) -> &IoContext {
        self.pimpl.as_ctx()
    }
}

impl<P> AsRawFd for DgramSocket<P> {
    fn as_raw_fd(&self) -> RawFd {
        self.pimpl.as_raw_fd()
    }
}

impl<P> Cancel for DgramSocket<P> {
    fn cancel(&self) {
        self.pimpl.cancel()
    }
}

impl<P> AsyncReadOp for DgramSocket<P>
where
    P: Protocol + 'static,
{
    fn add_read_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.pimpl.add_read_op(this, op, err)
    }

    fn next_read_op(&self, this: &mut ThreadIoContext) {
        self.pimpl.next_read_op(this)
    }
}

impl<P> AsyncWriteOp for DgramSocket<P>
where
    P: Protocol,
{
    fn add_write_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.pimpl.add_write_op(this, op, err)
    }

    fn next_write_op(&self, this: &mut ThreadIoContext) {
        self.pimpl.next_write_op(this)
    }
}

impl<P> fmt::Debug for DgramSocket<P>
where
    P: Protocol + fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}({})", self.protocol(), self.as_raw_fd())
    }
}

unsafe impl<P> Send for DgramSocket<P> {}

impl<P> Socket<P> for DgramSocket<P>
where
    P: Protocol,
{
    fn protocol(&self) -> &P {
        &self.pimpl.data
    }

    unsafe fn from_raw_fd(ctx: &IoContext, soc: RawFd, pro: P) -> Self {
        DgramSocket { pimpl: SocketImpl::new(ctx, soc, pro) }
    }
}
