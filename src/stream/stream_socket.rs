use super::IoStream;
use crate::ffi;
use crate::ops;
use crate::{IoContext, OsError, Protocol, Shutdown};
use std::os::fd::OwnedFd;
use std::time::Duration;

pub struct StreamSocketBuilder<P: Protocol> {
    ctx: IoContext,
    pro: P,
    conn_timeout: Duration,
}

impl<P> StreamSocketBuilder<P>
where
    P: Protocol,
{
    pub fn nb_connect(self, ep: &P::Endpoint) -> Result<StreamSocket<P>, OsError> {
        let soc = ffi::socket(self.pro)?;
        ffi::connect(&soc, ep)?;
        Ok(StreamSocket::new_priv(self.ctx, self.pro, soc))
    }

    pub fn connect(self, ep: &P::Endpoint) -> Result<StreamSocket<P>, OsError> {
        let soc = ops::connect(self.pro, ep, self.conn_timeout)?;
        Ok(StreamSocket::new_priv(self.ctx, self.pro, soc))
    }

    pub async fn async_connect(self, ep: &P::Endpoint) -> Result<StreamSocket<P>, OsError> {
        let soc = ops::async_connect(self.pro, ep, self.conn_timeout).await?;
        Ok(StreamSocket::new_priv(self.ctx, self.pro, soc))
    }
}

pub struct StreamSocket<P> {
    ctx: IoContext,
    pro: P,
    soc: OwnedFd,
    read_timeout: Duration,
    write_timeout: Duration,
}

impl<P> StreamSocket<P>
where
    P: Protocol,
{
    pub fn new(ctx: &IoContext, pro: P) -> StreamSocketBuilder<P> {
        StreamSocketBuilder {
            ctx: ctx.clone(),
            pro: pro,
            conn_timeout: Duration::MAX,
        }
    }

    pub(crate) fn new_priv(ctx: IoContext, pro: P, soc: OwnedFd) -> Self {
        Self {
            ctx: ctx,
            pro: pro,
            soc: soc,
            read_timeout: Duration::MAX,
            write_timeout: Duration::MAX,
        }
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn protocol(&self) -> P {
        self.pro
    }

    pub fn close(self) -> Result<(), OsError> {
        ffi::close(self.soc)
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<(), OsError> {
        ffi::shutdown(&self.soc, how)
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getsockname(&self.soc)
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getpeername(&self.soc)
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ffi::receive(&self.soc, buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ffi::send(&self.soc, buf)
    }

    pub fn write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::write_some(&self.ctx, &self.soc, buf, self.write_timeout)
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::send(&self.ctx, &self.soc, buf, self.write_timeout)
    }

    pub fn read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::read_some(&self.ctx, &self.soc, buf, self.read_timeout)
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::receive(&self.ctx, &self.soc, buf, self.read_timeout)
    }

    pub async fn async_read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::async_read_some(&self.ctx, &self.soc, buf, self.read_timeout).await
    }

    pub async fn async_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::async_send(&self.ctx, &self.soc, buf, self.write_timeout).await
    }

    pub async fn async_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::async_receive(&self.ctx, &self.soc, buf, self.read_timeout).await
    }

    pub async fn async_write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::async_write_some(&self.ctx, &self.soc, buf, self.write_timeout).await
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
