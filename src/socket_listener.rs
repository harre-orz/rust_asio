//

use executor::{IoContext, SocketContext, YieldContext, callback_socket};
use socket::{
    bind, close, getsockname, getsockopt, ioctl, listen, nb_accept, setsockopt, socket,
    wa_accept,
};
use socket_base::{
    GetSocketOption, IoControl, NativeHandle, Protocol, SetSocketOption, Socket, MAX_CONNECTIONS,
};
use std::io;
use std::sync::Arc;

struct Inner<P> {
    ctx: IoContext,
    soc: SocketContext,
    pro: P,
}

impl<P> Drop for Inner<P> {
    fn drop(&mut self) {
        self.ctx.deregister(&self.soc);
        let _ = close(self.soc.handle);
    }
}

#[derive(Clone)]
pub struct SocketListener<P> {
    inner: Arc<Inner<P>>,
}

impl<P> SocketListener<P>
where
    P: Protocol + Clone,
{
    pub fn new(ctx: &IoContext, pro: P) -> io::Result<Self> {
        Ok(socket(ctx, pro)?)
    }

    pub fn accept(&self) -> io::Result<(P::Socket, P::Endpoint)> {
        let mut wait = self.as_ctx().blocking();
        Ok(wa_accept(self, &self.inner.pro, &self.inner.ctx, &mut wait)?)
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.inner.ctx
    }

    pub fn async_accept(
        &self,
        yield_ctx: &mut YieldContext,
    ) -> io::Result<(P::Socket, P::Endpoint)> {
        Ok(wa_accept(self, &self.inner.pro, &self.inner.ctx, yield_ctx)?)
    }

    pub fn bind(&self, ep: &P::Endpoint) -> io::Result<()> {
        Ok(bind(self, ep)?)
    }

    pub fn close(self) -> io::Result<()> {
        self.inner.ctx.deregister(&self.inner.soc);
        Ok(close(self.native_handle())?)
    }

    pub fn listen(&self) -> io::Result<()> {
        Ok(listen(self, MAX_CONNECTIONS)?)
    }

    pub fn io_control<T>(&self, ctl: &mut T) -> io::Result<()>
    where
        T: IoControl,
    {
        Ok(ioctl(self.native_handle(), ctl)?)
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        Ok(getsockname(self, &self.inner.pro)?)
    }

    pub fn nb_accept(&self) -> io::Result<(P::Socket, P::Endpoint)> {
        Ok(nb_accept(self, self.inner.pro.clone(), &self.inner.ctx)?)
    }

    pub fn get_option<T>(&self) -> io::Result<T>
    where
        T: GetSocketOption<P>,
    {
        Ok(getsockopt(self, &self.inner.pro)?)
    }

    pub fn set_option<T>(&self, sockopt: T) -> io::Result<()>
    where
        T: SetSocketOption<P>,
    {
        Ok(setsockopt(self, &self.inner.pro, sockopt)?)
    }
}

impl<P> Socket<P> for SocketListener<P> {
    fn id(&self) -> usize {
        self.inner.soc.id()
    }

    fn is_stopped(&self) -> bool {
        self.inner.ctx.is_stopped()
    }

    fn native_handle(&self) -> NativeHandle {
        self.inner.soc.handle
    }

    unsafe fn unsafe_new(ctx: &IoContext, pro: P, handle: NativeHandle) -> Self {
        let inner = Arc::new(Inner {
            ctx: ctx.clone(),
            soc: SocketContext {
                handle: handle,
                callback: callback_socket,
            },
            pro: pro,
        });
        ctx.register(&inner.soc);
        SocketListener { inner: inner }
    }
}
