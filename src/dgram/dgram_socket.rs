use crate::error::OsError;
use crate::executor::{AsyncSocket, IoContext};
use crate::ffi::{self, ConnectedSocket};
use crate::ops;
use crate::socket_base::{Protocol, Shutdown};
use std::marker::PhantomData;
use std::time::Duration;

pub struct DgramSocketBuilder<'a, P> {
    ctx: &'a IoContext,
    soc: ConnectedSocket,
    pro: P,
    _marker: PhantomData<P>,
}

impl<'a, P> DgramSocketBuilder<'a, P>
where
    P: Protocol,
{
    pub fn new(ctx: &'a IoContext, pro: P) -> Result<Self, OsError> {
        let soc = ffi::socket(pro)?;
        Ok(DgramSocketBuilder {
            ctx: ctx,
            soc: soc,
            pro: pro,
            _marker: PhantomData,
        })
    }

    pub fn bind(self, ep: &P::Endpoint) -> Result<Self, OsError> {
        ffi::bind(&self.soc, ep)?;
        Ok(self)
    }

    pub fn ready(self) -> DgramSocket<P> {
        DgramSocket::new_priv(self.soc, self.pro, self.ctx)
    }

    pub fn async_ready(self) -> AsyncDgramSocket<P> {
        AsyncDgramSocket::new_priv(self.soc, self.pro, self.ctx)
    }

    pub fn connect(self, ep: &P::Endpoint) -> Result<DgramSocket<P>, OsError> {
        ffi::connect(&self.soc, ep)?;
        Ok(self.ready())
    }

    pub fn async_connect(self, ep: &P::Endpoint) -> Result<AsyncDgramSocket<P>, OsError> {
        ffi::connect(&self.soc, ep)?;
        Ok(self.async_ready())
    }
}

pub struct DgramSocket<P> {
    ctx: IoContext,
    soc: ConnectedSocket,
    pro: P,
    read_timeout: Duration,
    write_timeout: Duration,
}

impl<P> DgramSocket<P>
where
    P: Protocol,
{
    pub(crate) fn new_priv(soc: ConnectedSocket, pro: P, ctx: &IoContext) -> Self {
        Self {
            ctx: ctx.clone(),
            soc: soc,
            pro: pro,
            read_timeout: Duration::MAX,
            write_timeout: Duration::MAX,
        }
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn close(self) -> Result<(), OsError> {
        self.soc.close()
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getsockname(&self.soc)
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ffi::receive(&self.soc, buf)
    }

    pub fn nb_receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint), OsError> {
        ffi::receive_from(&self.soc, buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ffi::send(&self.soc, buf)
    }

    pub fn nb_send_to(&self, buf: &[u8], ep: &P::Endpoint) -> Result<usize, OsError> {
        ffi::send_to(&self.soc, buf, ep)
    }

    pub fn protocol(&self) -> P {
        self.pro
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<(), OsError> {
        ffi::shutdown(&self.soc, how)
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::receive(&self.ctx, &self.soc, buf, self.read_timeout)
    }

    pub fn receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint), OsError> {
        ops::receive_from(&self.ctx, &self.soc, buf, self.read_timeout)
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getpeername(&self.soc)
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::send(&self.ctx, &self.soc, buf, self.write_timeout)
    }

    pub fn send_to(&self, buf: &[u8], ep: &P::Endpoint) -> Result<usize, OsError> {
        ops::send_to(&self.ctx, &self.soc, buf, ep, self.write_timeout)
    }
}

pub struct AsyncDgramSocket<P> {
    soc: AsyncSocket,
    pro: P,
    read_timeout: Duration,
    write_timeout: Duration,
}

impl<P> AsyncDgramSocket<P>
where
    P: Protocol,
{
    pub(crate) fn new_priv(soc: ConnectedSocket, pro: P, ctx: &IoContext) -> Self {
        let soc = ctx.async_socket(soc);
        Self {
            soc: soc,
            pro: pro,
            read_timeout: Duration::MAX,
            write_timeout: Duration::MAX,
        }
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.soc.as_ctx()
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getsockname(self.soc.as_socket())
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ffi::receive(self.soc.as_socket(), buf)
    }

    pub fn nb_receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint), OsError> {
        ffi::receive_from(self.soc.as_socket(), buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ffi::send(self.soc.as_socket(), buf)
    }

    pub fn nb_send_to(&self, buf: &[u8], ep: &P::Endpoint) -> Result<usize, OsError> {
        ffi::send_to(self.soc.as_socket(), buf, ep)
    }

    pub fn protocol(&self) -> P {
        self.pro
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<(), OsError> {
        ffi::shutdown(self.soc.as_socket(), how)
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::receive(
            self.soc.as_ctx(),
            self.soc.as_socket(),
            buf,
            self.read_timeout,
        )
    }

    pub fn receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint), OsError> {
        ops::receive_from(
            self.soc.as_ctx(),
            self.soc.as_socket(),
            buf,
            self.read_timeout,
        )
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getpeername(self.soc.as_socket())
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::send(
            self.soc.as_ctx(),
            self.soc.as_socket(),
            buf,
            self.write_timeout,
        )
    }

    pub fn send_to(&self, buf: &[u8], ep: &P::Endpoint) -> Result<usize, OsError> {
        ops::send_to(
            self.soc.as_ctx(),
            self.soc.as_socket(),
            buf,
            ep,
            self.write_timeout,
        )
    }

    pub async fn async_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::async_receive(&self.soc, buf, self.read_timeout).await
    }

    pub async fn async_receive_from(
        &self,
        buf: &mut [u8],
    ) -> Result<(usize, P::Endpoint), OsError> {
        ops::async_receive_from(&self.soc, buf, self.read_timeout).await
    }

    pub async fn async_sent(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::async_send(&self.soc, buf, self.read_timeout).await
    }

    pub async fn async_send_to(&self, buf: &mut [u8], ep: &P::Endpoint) -> Result<usize, OsError> {
        ops::async_send_to(&self.soc, buf, ep, self.read_timeout).await
    }
}
