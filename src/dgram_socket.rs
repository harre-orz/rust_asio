//

use executor::{IoContext, YieldContext};
use socket::{
    bind, getpeername, getsockname, getsockopt, ioctl, nb_connect, nb_receive, nb_receive_from, nb_send, nb_send_to, setsockopt,
    shutdown, socket, wa_receive, wa_receive_from, wa_send, wa_send_to,
};
use socket_base::{
    GetSocketOption, IoControl, NativeHandle, Protocol, SetSocketOption, Shutdown, Socket,
};
use std::io;

pub struct DgramSocket<P> {
    ctx: IoContext,
    soc: NativeHandle,
    pro: P,
}

#[doc(hidden)]
impl<P> Drop for DgramSocket<P> {
    fn drop(&mut self) {
        let _ = self.ctx.disposal(self);
    }
}

impl<P> DgramSocket<P>
where
    P: Protocol + Clone,
{
    pub fn new(ctx: &IoContext, pro: P) -> io::Result<Self> {
        Ok(socket(ctx, pro)?)
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn async_connect(
        &self,
        ep: &P::Endpoint,
        yield_ctx: &mut YieldContext,
    ) -> io::Result<()> {
        let _ = yield_ctx;
        Ok(nb_connect(self, ep)?)
    }

    pub fn async_receive(
        &self,
        buf: &mut [u8],
        flags: i32,
        yield_ctx: &mut YieldContext,
    ) -> io::Result<usize> {
        Ok(wa_receive(self, buf, flags, yield_ctx)?)
    }

    pub fn async_receive_from(
        &self,
        buf: &mut [u8],
        flags: i32,
        yield_ctx: &mut YieldContext,
    ) -> io::Result<(usize, P::Endpoint)> {
        Ok(wa_receive_from(
            self,
            buf,
            flags,
            &self.pro.clone(),
            yield_ctx,
        )?)
    }

    pub fn async_send(
        &self,
        buf: &[u8],
        flags: i32,
        yield_ctx: &mut YieldContext,
    ) -> io::Result<usize> {
        Ok(wa_send(self, buf, flags, yield_ctx)?)
    }

    pub fn async_send_to(
        &self,
        buf: &[u8],
        flags: i32,
        ep: &P::Endpoint,
        yield_ctx: &mut YieldContext,
    ) -> io::Result<usize> {
        Ok(wa_send_to(self, buf, flags, ep, yield_ctx)?)
    }

    pub fn bind(&self, ep: &P::Endpoint) -> io::Result<()> {
        Ok(bind(self, ep)?)
    }

    pub fn close(self) -> io::Result<()> {
        Ok(self.ctx.disposal(&self)?)
    }

    pub fn connect(&self, ep: &P::Endpoint) -> io::Result<()> {
        Ok(nb_connect(self, ep)?)
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

    pub fn nb_receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        Ok(nb_receive(self, buf, flags)?)
    }

    pub fn nb_receive_from(&self, buf: &mut [u8], flags: i32) -> io::Result<(usize, P::Endpoint)> {
        Ok(nb_receive_from(self, buf, flags, &self.pro)?)
    }

    pub fn nb_send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        Ok(nb_send(self, buf, flags)?)
    }

    pub fn nb_send_to(&self, buf: &[u8], flags: i32, ep: &P::Endpoint) -> io::Result<usize> {
        Ok(nb_send_to(self, buf, flags, ep)?)
    }

    pub fn get_option<T>(&self) -> io::Result<T>
    where
        T: GetSocketOption<P>,
    {
        Ok(getsockopt(self, &self.pro)?)
    }

    pub fn receive(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        let mut wait = self.as_ctx().blocking();
        Ok(wa_receive(self, buf, flags, &mut wait)?)
    }

    pub fn receive_from(&self, buf: &mut [u8], flags: i32) -> io::Result<(usize, P::Endpoint)> {
        let mut wait = self.as_ctx().blocking();
        Ok(wa_receive_from(
            self,
            buf,
            flags,
            &self.pro,
            &mut wait
        )?)
    }

    pub fn remote_endpoint(&self) -> io::Result<P::Endpoint> {
        Ok(getpeername(self, &self.pro)?)
    }

    pub fn send(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        let mut wait = self.as_ctx().blocking();
        Ok(wa_send(self, buf, flags, &mut wait)?)
    }

    pub fn send_to(&self, buf: &[u8], flags: i32, ep: &P::Endpoint) -> io::Result<usize> {
        let mut wait = self.as_ctx().blocking();
        Ok(wa_send_to(self, buf, flags, ep, &mut wait)?)
    }

    pub fn set_option<T>(&self, sockopt: T) -> io::Result<()>
    where
        T: SetSocketOption<P>,
    {
        Ok(setsockopt(self, &self.pro, sockopt)?)
    }

    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        Ok(shutdown(self, how as i32)?)
    }
}

impl<P> Socket<P> for DgramSocket<P> {
    fn is_stopped(&self) -> bool {
        self.ctx.is_stopped()
    }

    fn native_handle(&self) -> NativeHandle {
        self.soc
    }

    unsafe fn unsafe_new(ctx: &IoContext, pro: P, soc: NativeHandle) -> Self {
        ctx.placement(DgramSocket {
            ctx: ctx.clone(),
            soc: soc,
            pro: pro,
        })
    }
}
