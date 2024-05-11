use crate::error::OsError;
use crate::executor::{AsyncSocket, IoContext};
use crate::ffi::{self, ConnectedSocket, Timeout};
use crate::ops;
use crate::socket_base::{Protocol, Shutdown};
use std::marker::PhantomData;

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
            ctx,
            soc,
            pro,
            _marker: PhantomData,
        })
    }

    pub fn bind(self, ep: &P::Endpoint) -> Result<Self, OsError> {
        ffi::bind(&self.soc, ep)?;
        Ok(self)
    }

    pub fn connect(self, ep: &P::Endpoint) -> Result<DgramSocket<P>, OsError> {
        ffi::connect(&self.soc, ep)?;
        Ok(self.ready())
    }

    pub fn ready(self) -> DgramSocket<P> {
        DgramSocket {
            ctx: self.ctx.clone(),
            soc: self.soc,
            pro: self.pro,
            read_timeout: Timeout::new(),
            write_timeout: Timeout::new(),
        }
    }
}

pub struct DgramSocket<P> {
    ctx: IoContext,
    soc: ConnectedSocket,
    pro: P,
    read_timeout: Timeout,
    write_timeout: Timeout,
}

impl<P> DgramSocket<P>
where
    P: Protocol,
{
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
        ops::receive(&self.soc, buf, self.read_timeout, &self.ctx)
    }

    pub fn receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint), OsError> {
        ops::receive_from(&self.soc, buf, self.read_timeout, &self.ctx)
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getpeername(&self.soc)
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::send(&self.soc, buf, self.write_timeout, &self.ctx)
    }

    pub fn send_to(&self, buf: &[u8], ep: &P::Endpoint) -> Result<usize, OsError> {
        ops::send_to(&self.soc, buf, ep, self.write_timeout, &self.ctx)
    }
}

pub struct AsyncDgramSocket<P> {
    soc: AsyncSocket,
    pro: P,
    read_timeout: Timeout,
    write_timeout: Timeout,
}

impl<P> AsyncDgramSocket<P>
where
    P: Protocol,
{
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
            self.soc.as_socket(),
            buf,
            self.read_timeout,
            self.soc.as_ctx(),
        )
    }

    pub fn receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint), OsError> {
        ops::receive_from(
            self.soc.as_socket(),
            buf,
            self.read_timeout,
            self.soc.as_ctx(),
        )
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getpeername(self.soc.as_socket())
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::send(
            self.soc.as_socket(),
            buf,
            self.write_timeout,
            self.soc.as_ctx(),
        )
    }

    pub fn send_to(&self, buf: &[u8], ep: &P::Endpoint) -> Result<usize, OsError> {
        ops::send_to(
            self.soc.as_socket(),
            buf,
            ep,
            self.write_timeout,
            self.soc.as_ctx(),
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

impl<P> From<DgramSocket<P>> for AsyncDgramSocket<P> {
    fn from(soc: DgramSocket<P>) -> Self {
        let DgramSocket {
            ctx,
            soc,
            pro,
            read_timeout,
            write_timeout,
        } = soc;
        Self {
            soc: ctx.async_socket(soc),
            pro,
            read_timeout,
            write_timeout,
        }
    }
}
