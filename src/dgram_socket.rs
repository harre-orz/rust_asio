#![allow(unreachable_patterns)]

use prelude::*;
use ffi::*;
use core::{AsIoContext, InnerSocket, IoContext, Perform, ThreadIoContext};
use ops::*;
use socket_base;

use std::io;
use std::fmt;

pub struct DgramSocket<P> {
    inner: Box<InnerSocket<P>>,
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
        async_recv(self, buf, flags, handler)
    }

    pub fn async_receive_from<F>(&self, buf: &mut [u8], flags: i32, handler: F) -> F::Output
    where
        F: Handler<(usize, P::Endpoint), io::Error>,
    {
        async_recvfrom(self, buf, flags, handler)
    }

    pub fn async_send<F>(&self, buf: &[u8], flags: i32, handler: F) -> F::Output
    where
        F: Handler<usize, io::Error>,
    {
        async_send(self, buf, flags, handler)
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
        async_sendto(self, buf, flags, ep, handler)
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
        self.inner.cancel()
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

    pub fn io_control<C>(&self, cmd: &mut C) -> io::Result<()>
    where
        C: IoControl,
    {
        Ok(ioctl(self, cmd)?)
    }

    pub fn nonblocking_receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        nonblocking_recv(self, buf, flags)
    }

    pub fn nonblocking_receive_from(
        &self,
        buf: &mut [u8],
        flags: i32,
    ) -> io::Result<(usize, P::Endpoint)> {
        nonblocking_recvfrom(self, buf, flags)
    }

    pub fn nonblocking_send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        nonblocking_send(self, buf, flags)
    }

    pub fn nonblocking_send_to(
        &self,
        buf: &[u8],
        flags: i32,
        ep: &P::Endpoint,
    ) -> io::Result<usize> {
        nonblocking_sendto(self, buf, flags, ep)
    }

    pub fn receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        recv_timeout(self, buf, flags, &Timeout::default())
    }

    pub fn receive_from(&self, buf: &mut [u8], flags: i32) -> io::Result<(usize, P::Endpoint)> {
        recvfrom_timeout(self, buf, flags, &Timeout::default())
    }

    pub fn remote_endpoint(&self) -> io::Result<P::Endpoint> {
        Ok(getpeername(self)?)
    }

    pub fn send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        send_timeout(self, buf, flags, &Timeout::default())
    }

    pub fn send_to(&self, buf: &[u8], flags: i32, ep: &P::Endpoint) -> io::Result<usize> {
        sendto_timeout(self, buf, flags, ep, &Timeout::default())
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
}

unsafe impl<P> AsIoContext for DgramSocket<P> {
    fn as_ctx(&self) -> &IoContext {
        self.inner.as_ctx()
    }
}

impl<P> AsRawFd for DgramSocket<P> {
    fn as_raw_fd(&self) -> RawFd {
        self.inner.as_raw_fd()
    }
}

impl<P> AsyncReadOp for DgramSocket<P>
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

impl<P> AsyncWriteOp for DgramSocket<P>
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
        &self.inner.data
    }

    unsafe fn from_raw_fd(ctx: &IoContext, soc: RawFd, pro: P) -> Self {
        DgramSocket { inner: InnerSocket::new(ctx, soc, pro) }
    }
}
