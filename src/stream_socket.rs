//

use executor::{IoContext, SocketContext, YieldContext};
use socket::{
    bind, close, getpeername,
    getsockname, getsockopt, ioctl, nb_connect, nb_read_some, nb_receive, nb_send, nb_write_some,
    setsockopt, shutdown, socket,
    wa_connect, wa_read_some, wa_receive, wa_send, wa_write_some,
};
use socket_base::{
    BytesReadable, GetSocketOption, IoControl, NativeHandle, Protocol, SetSocketOption, Shutdown,
    Socket,
};
use Stream;

use std::io;
use std::sync::Arc;

struct Inner<P> {
    pro: P,
    ctx: IoContext,
    soc: SocketContext,
}

impl<P> Drop for Inner<P> {
    fn drop(&mut self) {
        self.soc.deregister(&self.ctx)
    }
}

#[derive(Clone)]
pub struct StreamSocket<P> {
    inner: Arc<Inner<P>>,
}

impl<P> StreamSocket<P>
where
    P: Protocol,
{
    pub fn new(ctx: &IoContext, pro: P) -> io::Result<Self> {
        Ok(socket(ctx, pro)?)
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.inner.ctx
    }

    pub fn async_connect(
        &self,
        ep: &P::Endpoint,
        yield_ctx: &mut YieldContext,
    ) -> io::Result<()> {
        Ok(wa_connect(self, ep, yield_ctx)?)
    }

    pub fn async_receive(
        &self,
        buf: &mut [u8],
        flags: i32,
        yield_ctx: &mut YieldContext,
    ) -> io::Result<usize> {
        Ok(wa_receive(self, buf, flags, yield_ctx)?)
    }

    pub fn async_send(
        &self,
        buf: &[u8],
        flags: i32,
        yield_ctx: &mut YieldContext,
    ) -> io::Result<usize> {
        Ok(wa_send(self, buf, flags, yield_ctx)?)
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
        let mut wait = self.as_ctx().blocking();
        Ok(wa_connect(self, ep, &mut wait)?)
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

    pub fn nb_receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        Ok(nb_receive(self, buf, flags)?)
    }

    pub fn nb_send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        Ok(nb_send(self, buf, flags)?)
    }

    pub fn get_option<T>(&self) -> io::Result<T>
    where
        T: GetSocketOption<P>,
    {
        Ok(getsockopt(self, &self.inner.pro)?)
    }

    pub fn receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        let mut wait = self.as_ctx().blocking();
        Ok(wa_receive(self, buf, flags, &mut wait)?)
    }

    pub fn remote_endpoint(&self) -> io::Result<P::Endpoint> {
        Ok(getpeername(self, &self.inner.pro)?)
    }

    pub fn send(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        let mut wait = self.as_ctx().blocking();
        Ok(wa_send(self, buf, flags, &mut wait)?)
    }

    pub fn set_option<T>(&self, sockopt: T) -> io::Result<()>
    where
        T: SetSocketOption<P>,
    {
        Ok(setsockopt(self, &self.inner.pro, sockopt)?)
    }

    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        Ok(shutdown(self, how as i32)?)
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

    unsafe fn unsafe_new(soc: NativeHandle, pro: P, ctx: &IoContext) -> Self {
        let inner = Arc::new(Inner {
            pro: pro,
            ctx: ctx.clone(),
            soc: SocketContext::socket(soc),
        });
        inner.soc.register(ctx);
        StreamSocket { inner: inner }
    }

    fn is_stopped(&self) -> bool {
        self.inner.ctx.is_stopped()
    }
}

impl<P: Protocol> Stream for StreamSocket<P> {
    type Error = io::Error;

    fn async_read_some(&self, buf: &mut [u8], yield_ctx: &mut YieldContext) -> Result<usize, Self::Error> {
        Ok(wa_read_some(self, buf, yield_ctx)?)
    }

    fn async_write_some(&self, buf: &[u8], yield_ctx: &mut YieldContext) -> Result<usize, Self::Error> {
        Ok(wa_write_some(self, buf, yield_ctx)?)
    }

    fn nb_read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        Ok(nb_read_some(self, buf)?)
    }

    fn nb_write_some(&self, buf: &[u8]) -> io::Result<usize> {
        Ok(nb_write_some(self, buf)?)
    }

    fn read_some(&self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let mut wait = self.as_ctx().blocking();
        Ok(wa_read_some(self, buf, &mut wait)?)
    }

    fn write_some(&self, buf: &[u8]) -> Result<usize, Self::Error> {
        let mut wait = self.as_ctx().blocking();
        Ok(wa_write_some(self, buf, &mut wait)?)
    }
}
