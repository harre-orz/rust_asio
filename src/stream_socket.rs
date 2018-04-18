use ffi::{AsRawFd, RawFd, SystemError, Timeout, socket, shutdown, bind, ioctl, getsockopt,
          setsockopt, getpeername, getsockname};
use core::{Protocol, Socket, IoControl, GetSocketOption, SetSocketOption, AsIoContext, SocketImpl,
           IoContext, Perform, ThreadIoContext, Cancel};
use handler::{Handler, AsyncReadOp, AsyncWriteOp};
use connect_ops::{async_connect, blocking_connect};
use read_ops::{Read, Recv, async_read_op, blocking_read_op, nonblocking_read_op};
use write_ops::{Sent, Write, async_write_op, blocking_write_op, nonblocking_write_op};
use stream::Stream;
use socket_base::{BytesReadable, Shutdown};

use std::io;
use std::fmt;
use std::time::Duration;

pub struct StreamSocket<P> {
    pimpl: Box<SocketImpl<P>>,
}

impl<P> StreamSocket<P>
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

    pub fn async_send<F>(&self, buf: &[u8], flags: i32, handler: F) -> F::Output
    where
        F: Handler<usize, io::Error>,
    {
        async_write_op(self, buf, handler, Sent::new(flags))
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
        blocking_connect(self, ep, &self.pimpl.timeout)
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        Ok(getsockname(self)?)
    }

    pub fn nonblocking_read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        nonblocking_read_op(self, buf, Read::new())
    }

    pub fn nonblocking_receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        nonblocking_read_op(self, buf, Recv::new(flags))
    }

    pub fn nonblocking_send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        nonblocking_write_op(self, buf, Sent::new(flags))
    }

    pub fn nonblocking_write_some(&self, buf: &[u8]) -> io::Result<usize> {
        nonblocking_write_op(self, buf, Write::new())
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

    pub fn read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        blocking_read_op(self, buf, &self.pimpl.timeout, Read::new())
    }

    pub fn receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        blocking_read_op(self, buf, &self.pimpl.timeout, Recv::new(flags))
    }

    pub fn send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        blocking_write_op(self, buf, &self.pimpl.timeout, Sent::new(flags))
    }

    pub fn remote_endpoint(&self) -> io::Result<P::Endpoint> {
        Ok(getpeername(self)?)
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

    pub fn write_some(&self, buf: &[u8]) -> io::Result<usize> {
        blocking_write_op(self, buf, &self.pimpl.timeout, Write::new())
    }
}

unsafe impl<P> AsIoContext for StreamSocket<P> {
    fn as_ctx(&self) -> &IoContext {
        self.pimpl.as_ctx()
    }
}

impl<P> AsRawFd for StreamSocket<P> {
    fn as_raw_fd(&self) -> RawFd {
        self.pimpl.as_raw_fd()
    }
}

impl<P> Cancel for StreamSocket<P> {
    fn cancel(&self) {
        self.pimpl.cancel()
    }
}

impl<P> AsyncReadOp for StreamSocket<P>
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

impl<P> AsyncWriteOp for StreamSocket<P>
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

impl<P> fmt::Debug for StreamSocket<P>
where
    P: Protocol + fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}({})", self.protocol(), self.as_raw_fd())
    }
}

impl<P> io::Read for StreamSocket<P>
where
    P: Protocol,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.read_some(buf)
    }
}

unsafe impl<P> Send for StreamSocket<P> {}

unsafe impl<P> Sync for StreamSocket<P> {}

impl<P> Stream for StreamSocket<P>
where
    P: Protocol,
{
    type Error = io::Error;

    fn async_read_some<F>(&self, buf: &[u8], handler: F) -> F::Output
    where
        F: Handler<usize, Self::Error>,
    {
        async_read_op(self, buf, handler, Read::new())
    }

    fn async_write_some<F>(&self, buf: &[u8], handler: F) -> F::Output
    where
        F: Handler<usize, Self::Error>,
    {
        async_write_op(self, buf, handler, Write::new())
    }
}

impl<P> Socket<P> for StreamSocket<P>
where
    P: Protocol,
{
    fn protocol(&self) -> &P {
        &self.pimpl.data
    }

    unsafe fn from_raw_fd(ctx: &IoContext, soc: RawFd, pro: P) -> Self {
        StreamSocket { pimpl: SocketImpl::new(ctx, soc, pro) }
    }
}

impl<P> io::Write for StreamSocket<P>
where
    P: Protocol,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_some(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
