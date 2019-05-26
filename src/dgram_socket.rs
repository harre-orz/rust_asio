//

use executor::{AsIoContext, IoContext, SocketContext, YieldContext};
use socket::{
    async_receive, async_receive_from, async_send, async_send_to, bind, bk_receive, bk_receive_from, bk_send,
    bk_send_to, close, getpeername, getsockname, getsockopt, ioctl, nb_connect, nb_receive, nb_receive_from, nb_send,
    nb_send_to, setsockopt, shutdown, socket, AsSocketContext, Timeout,
};
use socket_base::{GetSocketOption, IoControl, NativeHandle, Protocol, SetSocketOption, Shutdown, Socket};
use std::io;

pub struct DgramSocket<P> {
    ctx: IoContext,
    pro: P,
    soc: SocketContext,
    timeout: Timeout,
}

impl<P> DgramSocket<P>
where
    P: Protocol + Clone,
{
    pub fn new(ctx: &IoContext, pro: P) -> io::Result<Self> {
        Ok(socket(ctx, pro)?)
    }

    pub fn async_connect(&mut self, ep: &P::Endpoint, yield_ctx: &mut YieldContext) -> io::Result<()> {
        let _ = yield_ctx;
        Ok(nb_connect(self, ep)?)
    }

    pub fn async_receive(&mut self, buf: &mut [u8], flags: i32, yield_ctx: &mut YieldContext) -> io::Result<usize> {
        Ok(async_receive(self, buf, flags, yield_ctx)?)
    }

    pub fn async_receive_from(
        &mut self,
        buf: &mut [u8],
        flags: i32,
        yield_ctx: &mut YieldContext,
    ) -> io::Result<(usize, P::Endpoint)> {
        Ok(async_receive_from(self, buf, flags, &self.pro.clone(), yield_ctx)?)
    }

    pub fn async_send(&mut self, buf: &[u8], flags: i32, yield_ctx: &mut YieldContext) -> io::Result<usize> {
        Ok(async_send(self, buf, flags, yield_ctx)?)
    }

    pub fn async_send_to(
        &mut self,
        buf: &[u8],
        flags: i32,
        ep: &P::Endpoint,
        yield_ctx: &mut YieldContext,
    ) -> io::Result<usize> {
        Ok(async_send_to(self, buf, flags, ep, yield_ctx)?)
    }

    pub fn bind(&self, ep: &P::Endpoint) -> io::Result<()> {
        Ok(bind(self, ep)?)
    }

    pub fn close(self) -> io::Result<()> {
        Ok(close(&self)?)
    }

    pub fn connect(&self, ep: &P::Endpoint) -> io::Result<()> {
        Ok(nb_connect(self, ep)?)
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
        Ok(bk_receive(self, buf, flags, self.timeout)?)
    }

    pub fn receive_from(&self, buf: &mut [u8], flags: i32) -> io::Result<(usize, P::Endpoint)> {
        Ok(bk_receive_from(self, buf, flags, &self.pro, self.timeout)?)
    }

    pub fn remote_endpoint(&self) -> io::Result<P::Endpoint> {
        Ok(getpeername(self, &self.pro)?)
    }

    pub fn send(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        Ok(bk_send(self, buf, flags, self.timeout)?)
    }

    pub fn send_to(&self, buf: &[u8], flags: i32, ep: &P::Endpoint) -> io::Result<usize> {
        Ok(bk_send_to(self, buf, flags, ep, self.timeout)?)
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

impl<P> Drop for DgramSocket<P> {
    fn drop(&mut self) {
        let _ = close(self);
    }
}

impl<P> AsIoContext for DgramSocket<P> {
    fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }
}

impl<P> Socket<P> for DgramSocket<P> {
    fn native_handle(&self) -> NativeHandle {
        self.soc.native_handle()
    }

    unsafe fn unsafe_new(ctx: &IoContext, pro: P, soc: NativeHandle) -> Self {
        DgramSocket {
            ctx: ctx.clone(),
            pro: pro,
            soc: SocketContext::socket(soc),
            timeout: Timeout::new(),
        }
    }
}

impl<P> AsSocketContext for DgramSocket<P> {
    fn as_socket_ctx(&mut self) -> &mut SocketContext {
        &mut self.soc
    }
}
