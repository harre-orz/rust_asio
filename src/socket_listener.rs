//

use executor::{AsIoContext, IoContext, SocketContext, YieldContext};
use socket::{
    async_accept, bk_accept, close, getsockname, getsockopt, ioctl, listen, nb_accept, setsockopt, socket,
    AsSocketContext, Timeout,
};
use socket_base::{GetSocketOption, IoControl, NativeHandle, Protocol, SetSocketOption, Socket, MAX_CONNECTIONS};
use std::io;

pub struct SocketListener<P> {
    ctx: IoContext,
    pro: P,
    soc: SocketContext,
    timeout: Timeout,
}

impl<P> SocketListener<P>
where
    P: Protocol + Clone,
{
    pub fn new(ctx: &IoContext, pro: P) -> io::Result<Self> {
        Ok(socket(ctx, pro)?)
    }

    pub fn accept(&self) -> io::Result<(P::Socket, P::Endpoint)> {
        Ok(bk_accept(self, &self.pro, self.timeout)?)
    }

    pub fn async_accept(&mut self, yield_ctx: &mut YieldContext) -> io::Result<(P::Socket, P::Endpoint)> {
        Ok(async_accept(self, &self.pro.clone(), yield_ctx)?)
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
        Ok(getsockname(self, &self.pro)?)
    }

    pub fn nb_accept(&self) -> io::Result<(P::Socket, P::Endpoint)> {
        Ok(nb_accept(self, self.pro.clone())?)
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

impl<P> Drop for SocketListener<P> {
    fn drop(&mut self) {
        let _ = close(self);
    }
}

impl<P> AsIoContext for SocketListener<P> {
    fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }
}

impl<P> Socket<P> for SocketListener<P> {
    fn native_handle(&self) -> NativeHandle {
        self.soc.native_handle()
    }
    unsafe fn unsafe_new(ctx: &IoContext, pro: P, soc: NativeHandle) -> Self {
        SocketListener {
            ctx: ctx.clone(),
            pro: pro,
            soc: SocketContext::socket(soc),
            timeout: Timeout::new(),
        }
    }
}

impl<P> AsSocketContext for SocketListener<P> {
    fn as_socket_ctx(&mut self) -> &mut SocketContext {
        &mut self.soc
    }
}
