use crate::error::OsError;
use crate::executor::{AsyncSocket, IoContext};
use crate::ffi::{self, ConnectedSocket};
use crate::ops;
use crate::socket_base::{Protocol, Shutdown};
use std::time::Duration;

pub struct SeqPacketSocketBuilder<'a, P: Protocol> {
    ctx: &'a IoContext,
    soc: ConnectedSocket,
    pro: P,
}

impl<'a, P> SeqPacketSocketBuilder<'a, P>
where
    P: Protocol,
{
    pub fn new(ctx: &'a IoContext, pro: P) -> Result<Self, OsError> {
        let soc = ffi::socket(pro)?;
        Ok(Self {
            ctx: ctx,
            soc: soc,
            pro: pro,
        })
    }

    pub fn bind(self, ep: &P::Endpoint) -> Result<Self, OsError> {
        ffi::bind(&self.soc, ep)?;
        Ok(self)
    }

    pub fn ready(self) -> SeqPacketSocket<P> {
        SeqPacketSocket::new_priv(self.soc, self.pro, self.ctx)
    }

    pub fn async_ready(self) -> AsyncSeqPacketSocket<P> {
        let soc = self.ctx.async_socket(self.soc);
        AsyncSeqPacketSocket::new_priv(soc, self.pro)
    }

    pub fn connect(self, ep: &P::Endpoint) -> Result<SeqPacketSocket<P>, OsError> {
        ffi::connect(&self.soc, ep)?;
        Ok(self.ready())
    }

    pub fn async_connect(self, ep: &P::Endpoint) -> Result<AsyncSeqPacketSocket<P>, OsError> {
        ffi::connect(&self.soc, ep)?;
        Ok(self.async_ready())
    }
}

pub struct SeqPacketSocket<P> {
    ctx: IoContext,
    soc: ConnectedSocket,
    pro: P,
    read_timeout: Duration,
    write_timeout: Duration,
}

impl<P> SeqPacketSocket<P>
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

    pub fn local_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getsockname(&self.soc)
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ffi::receive(&self.soc, buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ffi::send(&self.soc, buf)
    }

    pub fn close(self) -> Result<(), OsError> {
        self.soc.close()
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

    pub fn remote_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getpeername(&self.soc)
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::send(&self.ctx, &self.soc, buf, self.write_timeout)
    }
}

pub struct AsyncSeqPacketSocket<P> {
    soc: AsyncSocket,
    pro: P,
    read_timeout: Duration,
    write_timeout: Duration,
}

impl<P> AsyncSeqPacketSocket<P>
where
    P: Protocol,
{
    pub(crate) fn new_priv(soc: AsyncSocket, pro: P) -> Self {
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

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ffi::send(self.soc.as_socket(), buf)
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

    pub async fn async_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::async_receive(&self.soc, buf, self.read_timeout).await
    }

    pub async fn async_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::async_send(&self.soc, buf, self.write_timeout).await
    }
}
