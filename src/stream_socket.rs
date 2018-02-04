#![allow(unreachable_patterns)]

use prelude::*;
use ffi::*;
use core::{AsIoContext, InnerSocket, IoContext, Perform, ThreadIoContext};
use handler::Handler;
use ops::*;
use streams::Stream;
use socket_base;

use std::io;
use std::fmt;

pub struct StreamSocket<P> {
    inner: Box<InnerSocket<P>>,
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
        async_recv(self, buf, flags, handler)
    }

    pub fn async_send<F>(&self, buf: &[u8], flags: i32, handler: F) -> F::Output
    where
        F: Handler<usize, io::Error>,
    {
        async_send(self, buf, flags, handler)
    }

    pub fn available(&self) -> io::Result<usize> {
        let mut bytes = socket_base::BytesReadable::default();
        ioctl(self, &mut bytes)?;
        Ok(bytes.get())
    }

    pub fn bind(&self, ep: &P::Endpoint) -> io::Result<()> {
        Ok(bind(self, ep)?)
    }

    pub fn cancel(&self) {
        self.inner.cancel();
    }

    pub fn connect(&self, ep: &P::Endpoint) -> io::Result<()> {
        connect_timeout(self, ep, &Timeout::default())
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        Ok(getsockname(self)?)
    }

    pub fn nonblocking_read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        nonblocking_read(self, buf)
    }

    pub fn nonblocking_receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        nonblocking_recv(self, buf, flags)
    }

    pub fn nonblocking_send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        nonblocking_send(self, buf, flags)
    }

    pub fn nonblocking_write_some(&self, buf: &[u8]) -> io::Result<usize> {
        nonblocking_write(self, buf)
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

    pub fn read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        read_timeout(self, buf, &Timeout::default())
    }

    pub fn receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        recv_timeout(self, buf, flags, &Timeout::default())
    }

    pub fn send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        send_timeout(self, buf, flags, &Timeout::default())
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

    pub fn shutdown(&self, how: socket_base::Shutdown) -> io::Result<()> {
        Ok(shutdown(self, how)?)
    }

    pub fn write_some(&self, buf: &[u8]) -> io::Result<usize> {
        write_timeout(self, buf, &Timeout::default())
    }
}

unsafe impl<P> AsIoContext for StreamSocket<P> {
    fn as_ctx(&self) -> &IoContext {
        self.inner.as_ctx()
    }
}

impl<P> AsRawFd for StreamSocket<P> {
    fn as_raw_fd(&self) -> RawFd {
        self.inner.as_raw_fd()
    }
}

impl<P> AsyncReadOp for StreamSocket<P>
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

impl<P> AsyncWriteOp for StreamSocket<P>
where
    P: Protocol,
{
    fn add_write_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.inner.add_write_op(this, op, err)
    }

    fn next_write_op(&self, this: &mut ThreadIoContext) {
        self.inner.next_write_op(this)
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
        async_read(self, buf, handler)
    }

    fn async_write_some<F>(&self, buf: &[u8], handler: F) -> F::Output
    where
        F: Handler<usize, Self::Error>,
    {
        async_write(self, buf, handler)
    }
}

impl<P> Socket<P> for StreamSocket<P>
where
    P: Protocol,
{
    fn protocol(&self) -> &P {
        &self.inner.data
    }

    unsafe fn from_raw_fd(ctx: &IoContext, soc: RawFd, pro: P) -> Self {
        StreamSocket {
            inner: InnerSocket::new(ctx, soc, pro),
        }
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
