use crate::error::OsError;
use crate::executor::{AsyncSocket, IoContext};
use crate::io_stream::{AsyncIoStream, IoStream};
use crate::ops;
use crate::socket::ffi::{self, Socket};
use crate::socket_base::{Protocol, Shutdown};
use std::cell::Cell;
use std::time::{Duration, Instant};

pub struct StreamSocketBuilder<'a, P: Protocol> {
    ctx: &'a IoContext,
    soc: Socket,
    pro: P,
    cto: Cell<Option<Instant>>,
}

impl<'a, P> StreamSocketBuilder<'a, P>
where
    P: Protocol,
{
    pub fn expires_at(&self, time: Instant) {
        self.cto.set(Some(time))
    }

    pub fn expires_from_now(&self, time: Duration) {
        self.expires_at(Instant::now() + time)
    }

    pub fn nb_connect(self, ep: &P::Endpoint) -> Result<StreamSocket<P>, OsError> {
        ffi::connect(&self.soc, ep)?;
        Ok(StreamSocket::new_priv(self.ctx, self.soc, self.pro))
    }

    pub fn connect(self, ep: &P::Endpoint) -> Result<StreamSocket<P>, OsError> {
        ops::connect(&self.soc, ep, self.ctx, self.cto.get())?;
        Ok(StreamSocket::new_priv(self.ctx, self.soc, self.pro))
    }

    pub async fn async_connect(self, ep: &P::Endpoint) -> Result<AsyncStreamSocket<P>, OsError> {
        let soc = AsyncSocket::new(self.ctx.clone(), self.soc);
        ops::async_connect(&soc, ep).await?;
        Ok(AsyncStreamSocket {
            soc: soc,
            pro: self.pro,
        })
    }
}

pub struct StreamSocket<P> {
    ctx: IoContext,
    soc: Socket,
    pro: P,
    exp: Cell<Option<Instant>>,
}

impl<P> StreamSocket<P>
where
    P: Protocol,
{
    pub fn new(ctx: &IoContext, pro: P) -> Result<StreamSocketBuilder<P>, OsError> {
        let soc = ffi::socket(pro)?;
        Ok(StreamSocketBuilder {
            ctx: ctx,
            soc: soc,
            pro: pro,
            cto: Cell::new(None),
        })
    }

    pub(crate) fn new_priv(ctx: &IoContext, soc: Socket, pro: P) -> Self {
        Self {
            ctx: ctx.clone(),
            soc: soc,
            pro: pro,
            exp: Cell::new(None),
        }
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn close(self) -> Result<(), OsError> {
        ffi::close(self.soc)
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
        ops::read_some(&self.soc, buf, &self.ctx, self.exp.get())
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::receive(&self.soc, buf, &self.ctx, self.exp.get())
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getpeername(&self.soc)
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::send(&self.soc, buf, &self.ctx, self.exp.get())
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<(), OsError> {
        ffi::shutdown(&self.soc, how)
    }

    pub fn write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::write_some(&self.soc, buf, &self.ctx, self.exp.get())
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
}

impl<P> AsyncStreamSocket<P>
where
    P: Protocol,
{
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

    pub fn remote_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getpeername(&self.soc.as_socket())
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<(), OsError> {
        ffi::shutdown(&self.soc.as_socket(), how)
    }

    pub async fn async_read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::async_read_some(&self.soc, buf).await
    }

    pub async fn async_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::async_receive(&self.soc, buf).await
    }

    pub async fn async_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::async_send(&self.soc, buf).await
    }

    pub async fn async_write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::async_write_some(&self.soc, buf).await
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

impl<P> From<StreamSocket<P>> for AsyncStreamSocket<P> {
    fn from(soc: StreamSocket<P>) -> Self {
        Self {
            soc: AsyncSocket::new(soc.ctx, soc.soc),
            pro: soc.pro,
        }
    }
}
