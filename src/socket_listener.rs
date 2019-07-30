//

use executor::{AsIoContext, IoContext, SocketContext, YieldContext};
use socket::{
    bk_accept, close, getsockname, getsockopt, ioctl, listen, nb_accept, setsockopt, socket,
    Blocking,
};
use socket_base::{
    GetSocketOption, IoControl, NativeHandle, Protocol, SetSocketOption, Socket, MAX_CONNECTIONS,
};
use std::io;
use std::time::Duration;

struct Inner<P> {
    ctx: IoContext,
    soc: SocketContext,
    pro: P,
    blk: Blocking,
}

pub struct SocketListener<P> {
    inner: Box<Inner<P>>,
}

impl<P> SocketListener<P>
where
    P: Protocol + Clone,
{
    pub fn new(ctx: &IoContext, pro: P) -> io::Result<Self> {
        Ok(socket(ctx, pro)?)
    }

    pub fn accept(&self) -> io::Result<(P::Socket, P::Endpoint)> {
        let mut blk = self.inner.blk.clone();
        Ok(bk_accept(self, &self.inner.pro, &mut blk)?)
    }

    pub fn async_accept(
        &mut self,
        yield_ctx: &mut YieldContext,
    ) -> io::Result<(P::Socket, P::Endpoint)> {
        Ok(bk_accept(self, &self.inner.pro, yield_ctx)?)
    }

    pub fn close(self) -> io::Result<()> {
        Ok(close(&self)?)
    }

    pub fn listen(&self) -> io::Result<()> {
        Ok(listen(self, MAX_CONNECTIONS)?)
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

    pub fn nb_accept(&self) -> io::Result<(P::Socket, P::Endpoint)> {
        Ok(nb_accept(self, self.inner.pro.clone())?)
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

    pub fn set_option<T>(&self, sockopt: T) -> io::Result<()>
    where
        T: SetSocketOption<P>,
    {
        Ok(setsockopt(self, &self.inner.pro, sockopt)?)
    }

    pub fn set_timeout(&mut self, timeout: Duration) -> io::Result<()> {
        Ok(self.inner.blk.set_timeout(timeout)?)
    }
}

impl<P> Drop for SocketListener<P> {
    fn drop(&mut self) {
        let _ = close(self);
    }
}

impl<P> AsIoContext for SocketListener<P> {
    fn as_ctx(&self) -> &IoContext {
        &self.inner.ctx
    }
}

impl<P> Socket<P> for SocketListener<P> {
    #[doc(hidden)]
    fn as_inner(&self) -> &SocketContext {
        &self.inner.soc
    }

    fn native_handle(&self) -> NativeHandle {
        self.inner.soc.native_handle()
    }

    unsafe fn unsafe_new(ctx: &IoContext, pro: P, soc: NativeHandle) -> Self {
        SocketListener {
            inner: Box::new(Inner {
                ctx: ctx.clone(),
                pro: pro,
                soc: SocketContext::socket(soc),
                blk: Blocking::new(),
            }),
        }
    }
}
