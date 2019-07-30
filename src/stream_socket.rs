//

use executor::{AsIoContext, IoContext, SocketContext, YieldContext};
use socket::{
    bind, bk_connect, bk_read_some, bk_receive, bk_send, bk_write_some, close, getpeername,
    getsockname, getsockopt, ioctl, nb_connect, nb_read_some, nb_receive, nb_send, nb_write_some,
    setsockopt, shutdown, socket, Blocking,
};
use socket_base::{
    BytesReadable, GetSocketOption, IoControl, NativeHandle, Protocol, SetSocketOption, Shutdown,
    Socket,
};
use std::io;
use std::time::Duration;

struct Inner<P> {
    ctx: IoContext,
    soc: SocketContext,
    pro: P,
    blk: Blocking,
}

impl<P> Drop for Inner<P> {
    fn drop(&mut self) {}
}

pub struct StreamSocket<P> {
    inner: Box<Inner<P>>,
}

impl<P> StreamSocket<P>
where
    P: Protocol,
{
    pub fn new(ctx: &IoContext, pro: P) -> io::Result<Self> {
        Ok(socket(ctx, pro)?)
    }

    pub fn async_connect(
        &mut self,
        ep: &P::Endpoint,
        yield_ctx: &mut YieldContext,
    ) -> io::Result<()> {
        Ok(bk_connect(self, ep, yield_ctx)?)
    }

    pub fn async_read_some(
        &mut self,
        buf: &mut [u8],
        yield_ctx: &mut YieldContext,
    ) -> io::Result<usize> {
        Ok(bk_read_some(self, buf, yield_ctx)?)
    }

    pub fn async_receive(
        &mut self,
        buf: &mut [u8],
        flags: i32,
        yield_ctx: &mut YieldContext,
    ) -> io::Result<usize> {
        Ok(bk_receive(self, buf, flags, yield_ctx)?)
    }

    pub fn async_send(
        &mut self,
        buf: &[u8],
        flags: i32,
        yield_ctx: &mut YieldContext,
    ) -> io::Result<usize> {
        Ok(bk_send(self, buf, flags, yield_ctx)?)
    }

    pub fn async_write_some(
        &mut self,
        buf: &[u8],
        yield_ctx: &mut YieldContext,
    ) -> io::Result<usize> {
        Ok(bk_write_some(self, buf, yield_ctx)?)
    }

    pub fn available(&self) -> io::Result<usize> {
        let mut readable = BytesReadable::new();
        ioctl(self, &mut readable)?;
        Ok(readable.get())
    }

    pub fn bind(&self, ep: &P::Endpoint) -> io::Result<()> {
        Ok(bind(self, ep)?)
    }

    pub fn connect(&self, ep: &P::Endpoint) -> io::Result<()> {
        let mut blk = self.inner.blk.clone();
        Ok(bk_connect(self, ep, &mut blk)?)
    }

    pub fn close(self) -> io::Result<()> {
        Ok(close(&self)?)
    }

    pub fn io_control<T>(&self, ctl: &mut T) -> io::Result<()>
    where
        T: IoControl,
    {
        Ok(ioctl(self, ctl)?)
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        Ok(getsockname(self, &self.inner.pro)?)
    }

    pub fn nb_connect(&self, ep: &P::Endpoint) -> io::Result<()> {
        Ok(nb_connect(self, ep)?)
    }

    pub fn nb_read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        Ok(nb_read_some(self, buf)?)
    }

    pub fn nb_receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        Ok(nb_receive(self, buf, flags)?)
    }

    pub fn nb_send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        Ok(nb_send(self, buf, flags)?)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> io::Result<usize> {
        Ok(nb_write_some(self, buf)?)
    }

    pub fn get_option<T>(&self) -> io::Result<T>
    where
        T: GetSocketOption<P>,
    {
        Ok(getsockopt(self, &self.inner.pro)?)
    }

    pub fn get_timeout(&self) -> Duration {
        self.inner.blk.get_timeout()
    }

    pub fn read_some(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut blk = self.inner.blk.clone();
        Ok(bk_read_some(self, buf, &mut blk)?)
    }

    pub fn receive(&mut self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        let mut blk = self.inner.blk.clone();
        Ok(bk_receive(self, buf, flags, &mut blk)?)
    }

    pub fn remote_endpoint(&self) -> io::Result<P::Endpoint> {
        Ok(getpeername(self, &self.inner.pro)?)
    }

    pub fn send(&mut self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        let mut blk = self.inner.blk.clone();
        Ok(bk_send(self, buf, flags, &mut blk)?)
    }

    pub fn set_option<T>(&self, sockopt: T) -> io::Result<()>
    where
        T: SetSocketOption<P>,
    {
        Ok(setsockopt(self, &self.inner.pro, sockopt)?)
    }

    pub fn set_timeout(&mut self, timeout: Duration) -> io::Result<()> {
        Ok(self.inner.blk.set_timeout(timeout)?)
    }

    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        Ok(shutdown(self, how as i32)?)
    }

    pub fn write_some(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut blk = self.inner.blk.clone();
        Ok(bk_write_some(self, buf, &mut blk)?)
    }
}

impl<P> AsIoContext for StreamSocket<P> {
    fn as_ctx(&self) -> &IoContext {
        &self.inner.ctx
    }
}

impl<P> Socket<P> for StreamSocket<P> {
    #[doc(hidden)]
    fn as_inner(&self) -> &SocketContext {
        &self.inner.soc
    }

    fn native_handle(&self) -> NativeHandle {
        self.inner.soc.native_handle()
    }

    unsafe fn unsafe_new(ctx: &IoContext, pro: P, soc: NativeHandle) -> Self {
        let inner = Box::new(Inner {
            ctx: ctx.clone(),
            soc: SocketContext::socket(soc),
            pro: pro,
            blk: Blocking::new(),
        });
        inner.soc.register(ctx);
        StreamSocket { inner: inner }
    }
}
