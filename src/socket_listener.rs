//

use executor::{IoContext, YieldContext};
use socket::{
    bind, getsockname, getsockopt, ioctl, listen, nb_accept, setsockopt, socket,
    wa_accept,
};
use socket_base::{
    GetSocketOption, IoControl, NativeHandle, Protocol, SetSocketOption, Socket, MAX_CONNECTIONS,
};
use std::io;

pub struct SocketListener<P> {
    ctx: IoContext,
    soc: NativeHandle,
    pro: P,
}

#[doc(hidden)]
impl<P> Drop for SocketListener<P> {
    fn drop(&mut self) {
        let _ = self.ctx.disposal(self);
    }
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
        Ok(wa_accept(self, &self.pro, &self.ctx, &mut wait)?)
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn async_accept(
        &self,
        yield_ctx: &mut YieldContext,
    ) -> io::Result<(P::Socket, P::Endpoint)> {
        Ok(wa_accept(self, &self.pro, &self.ctx, yield_ctx)?)
    }

    pub fn bind(&self, ep: &P::Endpoint) -> io::Result<()> {
        Ok(bind(self, ep)?)
    }

    pub fn close(self) -> io::Result<()> {
        Ok(self.ctx.disposal(&self)?)
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
        Ok(getsockname(self, &self.pro)?)
    }

    pub fn nb_accept(&self) -> io::Result<(P::Socket, P::Endpoint)> {
        Ok(nb_accept(self, self.pro.clone(), &self.ctx)?)
    }

    pub fn get_option<T>(&self) -> io::Result<T>
    where
        T: GetSocketOption<P>,
    {
        Ok(getsockopt(self, &self.pro)?)
    }

    pub fn set_option<T>(&self, sockopt: T) -> io::Result<()>
    where
        T: SetSocketOption<P>,
    {
        Ok(setsockopt(self, &self.pro, sockopt)?)
    }
}

impl<P> Socket<P> for SocketListener<P> {
    fn is_stopped(&self) -> bool {
        self.ctx.is_stopped()
    }

    fn native_handle(&self) -> NativeHandle {
        self.soc
    }

    unsafe fn unsafe_new(ctx: &IoContext, pro: P, soc: NativeHandle) -> Self {
        ctx.placement(SocketListener {
            ctx: ctx.clone(),
            soc: soc,
            pro: pro,
        })
    }
}
