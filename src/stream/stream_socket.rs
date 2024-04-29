use super::{AsyncIoStream, IoStream};
use crate::error::OsError;
use crate::executor::{AsyncSocket, IoContext};
use crate::ffi::{self, ConnectedSocket};
use crate::ops;
use crate::socket_base::{Protocol, Shutdown};
use std::time::Duration;

pub struct StreamSocketBuilder<'a, P: Protocol> {
    ctx: &'a IoContext,
    soc: ConnectedSocket,
    pro: P,
    conn_timeout: Duration,
}

impl<'a, P> StreamSocketBuilder<'a, P>
where
    P: Protocol,
{
    pub fn new(ctx: &'a IoContext, pro: P) -> Result<Self, OsError> {
        let soc = ffi::socket(pro)?;
        Ok(Self {
            ctx: ctx,
            soc: soc,
            pro: pro,
            conn_timeout: Duration::MAX,
        })
    }

    pub fn nb_connect(self, ep: &P::Endpoint) -> Result<StreamSocket<P>, OsError> {
        ffi::connect(&self.soc, ep)?;
        Ok(StreamSocket::new_priv(self.soc, self.pro, self.ctx))
    }

    pub fn connect(self, ep: &P::Endpoint) -> Result<StreamSocket<P>, OsError> {
        let soc = ops::connect(&self.ctx, self.pro, ep, self.conn_timeout)?;
        Ok(StreamSocket::new_priv(soc, self.pro, self.ctx))
    }

    pub async fn async_connect(self, ep: &P::Endpoint) -> Result<AsyncStreamSocket<P>, OsError> {
        let soc = ops::async_connect(self.ctx, self.pro, ep, self.conn_timeout).await?;
        Ok(AsyncStreamSocket::new_priv(soc, self.pro))
    }
}

pub struct StreamSocket<P> {
    ctx: IoContext,
    soc: ConnectedSocket,
    pro: P,
    read_timeout: Duration,
    write_timeout: Duration,
}

impl<P> StreamSocket<P>
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

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ffi::read(&self.soc, buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ffi::send(&self.soc, buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        ffi::write(&self.soc, buf)
    }

    pub fn protocol(&self) -> P {
        self.pro
    }

    pub fn read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::read_some(&self.ctx, &self.soc, buf, self.read_timeout)
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

    pub fn shutdown(&self, how: Shutdown) -> Result<(), OsError> {
        ffi::shutdown(&self.soc, how)
    }

    pub fn write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::write_some(&self.ctx, &self.soc, buf, self.write_timeout)
    }
}

impl<P> IoStream for StreamSocket<P>
where
    P: Protocol,
{
    type Error = OsError;

    fn read(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.read_some(buf)
    }

    fn write(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.write_some(buf)
    }
}

pub struct AsyncStreamSocket<P> {
    soc: AsyncSocket,
    pro: P,
    read_timeout: Duration,
    write_timeout: Duration,
}

impl<P> AsyncStreamSocket<P>
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

    pub fn local_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getsockname(&self.soc.as_socket())
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ffi::receive(&self.soc.as_socket(), buf)
    }

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ffi::read(&self.soc.as_socket(), buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ffi::send(&self.soc.as_socket(), buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        ffi::write(&self.soc.as_socket(), buf)
    }

    pub fn protocol(&self) -> P {
        self.pro
    }

    pub fn read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::read_some(
            &self.soc.as_ctx(),
            &self.soc.as_socket(),
            buf,
            self.read_timeout,
        )
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::receive(
            &self.soc.as_ctx(),
            &self.soc.as_socket(),
            buf,
            self.read_timeout,
        )
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getpeername(&self.soc.as_socket())
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::send(
            &self.soc.as_ctx(),
            &self.soc.as_socket(),
            buf,
            self.write_timeout,
        )
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<(), OsError> {
        ffi::shutdown(&self.soc.as_socket(), how)
    }

    pub fn write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::write_some(
            &self.soc.as_ctx(),
            &self.soc.as_socket(),
            buf,
            self.write_timeout,
        )
    }

    pub async fn async_read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::async_read_some(&self.soc, buf, self.read_timeout).await
    }

    pub async fn async_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::async_receive(&self.soc, buf, self.read_timeout).await
    }

    pub async fn async_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::async_send(&self.soc, buf, self.write_timeout).await
    }

    pub async fn async_write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::async_write_some(&self.soc, buf, self.write_timeout).await
    }
}

impl<P> IoStream for AsyncStreamSocket<P>
where
    P: Protocol,
{
    type Error = OsError;

    fn read(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.read_some(buf)
    }

    fn write(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.write_some(buf)
    }
}

impl<P> AsyncIoStream for AsyncStreamSocket<P>
where
    P: Protocol,
{
    type Error = OsError;

    async fn async_read(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.async_read_some(buf).await
    }

    async fn async_write(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.async_write_some(buf).await
    }
}
