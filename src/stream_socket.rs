use crate::IoContext;
use crate::error::OsError;
use crate::exec::async_socket::AsyncSocket;
use crate::ffi::socket::Socket;
use crate::io_stream::{AsyncIoStream, IoStream};
use crate::ops;
use crate::socket_base::{Protocol, Shutdown};
use std::cell::Cell;
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, OsError>;

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
    pub fn expires_at(&self, cto: Instant) {
        self.cto.set(Some(cto))
    }

    pub fn expires_from_now(&self, cto: Duration) {
        self.expires_at(Instant::now() + cto)
    }

    pub fn nb_connect(self, ep: &P::Endpoint) -> Result<StreamSocket<P>> {
        self.soc.nb_connect(ep)?;
        Ok(StreamSocket::new_priv(self.ctx, self.soc, self.pro))
    }

    pub fn connect(self, ep: &P::Endpoint) -> Result<StreamSocket<P>> {
        ops::connect(&self.soc, ep, self.ctx, self.cto.get())?;
        Ok(StreamSocket::new_priv(self.ctx, self.soc, self.pro))
    }

    pub async fn async_connect(self, ep: &P::Endpoint) -> Result<AsyncStreamSocket<P>> {
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
    cto: Cell<Option<Instant>>,
}

impl<P> StreamSocket<P>
where
    P: Protocol,
{
    pub fn new(ctx: &IoContext, pro: P) -> Result<StreamSocketBuilder<P>> {
        let soc = Socket::new(pro)?;
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
            cto: Cell::new(None),
        }
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn close(self) -> Result<()> {
        self.soc.close()
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.getsockname()
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.nb_receive(buf)
    }

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.nb_read(buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.nb_send(buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize> {
        self.soc.nb_write(buf)
    }

    pub fn protocol(&self) -> P {
        self.pro
    }

    pub fn read_some(&self, buf: &mut [u8]) -> Result<usize> {
        ops::read_some(&self.soc, buf, &self.ctx, self.cto.get())
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize> {
        ops::receive(&self.soc, buf, &self.ctx, self.cto.get())
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.getpeername()
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize> {
        ops::send(&self.soc, buf, &self.ctx, self.cto.get())
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        self.soc.shutdown(how)
    }

    pub fn write_some(&self, buf: &[u8]) -> Result<usize> {
        ops::write_some(&self.soc, buf, &self.ctx, self.cto.get())
    }
}

impl<P> IoStream for StreamSocket<P>
where
    P: Protocol,
{
    type Error = OsError;

    fn read(&self, buf: &mut [u8]) -> Result<usize> {
        self.read_some(buf)
    }

    fn write(&self, buf: &[u8]) -> Result<usize> {
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
    pub fn local_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.as_socket().getsockname()
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.as_socket().nb_receive(buf)
    }

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.as_socket().nb_read(buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.as_socket().nb_send(buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize> {
        self.soc.as_socket().nb_write(buf)
    }

    pub fn protocol(&self) -> P {
        self.pro
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.as_socket().getpeername()
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        self.soc.as_socket().shutdown(how)
    }

    pub async fn async_read_some(&self, buf: &mut [u8]) -> Result<usize> {
        ops::async_read_some(&self.soc, buf).await
    }

    pub async fn async_receive(&self, buf: &mut [u8]) -> Result<usize> {
        ops::async_receive(&self.soc, buf).await
    }

    pub async fn async_send(&self, buf: &[u8]) -> Result<usize> {
        ops::async_send(&self.soc, buf).await
    }

    pub async fn async_write_some(&self, buf: &[u8]) -> Result<usize> {
        ops::async_write_some(&self.soc, buf).await
    }
}

impl<P> AsyncIoStream for AsyncStreamSocket<P>
where
    P: Protocol,
{
    type Error = OsError;

    async fn async_read(&self, buf: &mut [u8]) -> Result<usize> {
        self.async_read_some(buf).await
    }

    async fn async_write(&self, buf: &[u8]) -> Result<usize> {
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
