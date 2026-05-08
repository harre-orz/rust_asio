use crate::IoContext;
use crate::buffer::{AsyncIoStream, IoStream};
use crate::error::{OsError, Result};
use crate::exec::AsyncSocket;
use crate::ops::{self};
use crate::sockaddr::SockAddr;
use crate::socket::{Shutdown, Socket, Timeout};
use crate::socket_base::{Endpoints, Protocol};
use std::marker::PhantomData;
use std::time::Duration;

pub struct AsyncStreamSocket<P>
where
    P: Protocol,
{
    soc: AsyncSocket,
    timeout: Timeout,
    _marker: PhantomData<P>,
}

impl<P> AsyncStreamSocket<P>
where
    P: Protocol,
{
    pub const fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = Timeout::from_duration(timeout)
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.as_socket().getsockname()
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.as_socket().receive(buf)
    }

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.as_socket().read(buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.as_socket().send(buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize> {
        self.soc.as_socket().write(buf)
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.as_socket().getpeername()
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        self.soc.as_socket().shutdown(how)
    }

    pub async fn async_read_some(&self, buf: &mut [u8]) -> Result<usize> {
        ops::async_read_some(&self.soc, buf, self.timeout).await
    }

    pub async fn async_receive(&self, buf: &mut [u8]) -> Result<usize> {
        ops::async_receive(&self.soc, buf, self.timeout).await
    }

    pub async fn async_send(&self, buf: &[u8]) -> Result<usize> {
        ops::async_send(&self.soc, buf, self.timeout).await
    }

    pub async fn async_write_some(&self, buf: &[u8]) -> Result<usize> {
        ops::async_write_some(&self.soc, buf, self.timeout).await
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

pub struct StreamSocket<P>
where
    P: Protocol,
{
    soc: Socket,
    ctx: IoContext,
    timeout: Timeout,
    _marker: PhantomData<P>,
}

impl<P> StreamSocket<P>
where
    P: Protocol,
{
    pub(crate) fn new_impl(ctx: IoContext, soc: Socket) -> Self {
        Self {
            soc: soc,
            ctx: ctx,
            timeout: Timeout::infinite(),
            _marker: PhantomData,
        }
    }

    pub fn close(self) -> Result<()> {
        self.soc.close()
    }

    pub const fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = Timeout::from_duration(timeout)
    }
    pub fn local_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.getsockname()
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.receive(buf)
    }

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.read(buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.send(buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize> {
        self.soc.write(buf)
    }

    // pub fn protocol(&self) -> P {
    //     self.pro
    // }

    pub fn read_some(&self, buf: &mut [u8]) -> Result<usize> {
        ops::read_some(&self.ctx, &self.soc, buf, self.timeout)
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize> {
        ops::receive(&self.ctx, &self.soc, buf, self.timeout)
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.getpeername()
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize> {
        ops::send(&self.ctx, &self.soc, buf, self.timeout)
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        self.soc.shutdown(how)
    }

    pub fn write_some(&self, buf: &[u8]) -> Result<usize> {
        ops::write_some(&self.ctx, &self.soc, buf, self.timeout)
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

/// Converts Asynchronous socket.
///
/// # Examples
///
/// ```no_run
/// use asyncio::IoContext;
/// use asyncio::local::{LocalStreamEndpoint, LocalStreamSocket, AsyncLocalStreamSocket};
/// use std::path::Path;
///
/// let ctx = &IoContext::new().unwrap();
/// let ep = LocalStreamEndpoint::new(Path::new("/foo/bar")).unwrap();
/// let soc = LocalStreamSocket::new(ctx).connect(&ep).unwrap();
/// let soc = AsyncLocalStreamSocket::from(soc);
/// ```
impl<P> From<StreamSocket<P>> for AsyncStreamSocket<P>
where
    P: Protocol,
{
    fn from(soc: StreamSocket<P>) -> Self {
        Self {
            soc: AsyncSocket::new(soc.ctx, soc.soc),
            timeout: soc.timeout,
            _marker: PhantomData,
        }
    }
}

pub struct StreamSocketBuilder<P>
where
    P: Protocol,
{
    pro: P::Type,
    ctx: IoContext,
    timeout: Timeout,
}

impl<P: Protocol> StreamSocketBuilder<P> {
    pub(crate) fn new_impl(ctx: IoContext, pro: P::Type) -> Self {
        Self {
            pro: pro,
            ctx: ctx,
            timeout: Timeout::infinite(),
        }
    }

    pub fn nb_connect<'a, E>(self, eps: E) -> Result<StreamSocket<P>>
    where
        P: 'a,
        E: Endpoints<'a, P>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in eps.endpoints() {
            let pro = P::new(ep.sockaddr_ref().address_family(), self.pro);
            let soc = Socket::new(pro)?;
            match soc.connect(&ep) {
                Ok(_) => {
                    return Ok(StreamSocket::new_impl(self.ctx, soc));
                }
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }

    pub fn connect<'a, E>(self, eps: E) -> Result<StreamSocket<P>>
    where
        P: 'a,
        E: Endpoints<'a, P>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in eps.endpoints() {
            let pro = P::new(ep.sockaddr_ref().address_family(), self.pro);
            let soc = Socket::new(pro)?;
            match ops::connect(&self.ctx, &soc, &ep, self.timeout) {
                Ok(_) => {
                    return Ok(StreamSocket::new_impl(self.ctx, soc));
                }
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }

    pub fn protocol_type(&self) -> P::Type {
        self.pro
    }

    pub async fn async_connect<'a, E>(self, eps: E) -> Result<AsyncStreamSocket<P>>
    where
        P: 'a,
        E: Endpoints<'a, P>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in eps.endpoints() {
            let pro = P::new(ep.sockaddr_ref().address_family(), self.pro);
            let soc = Socket::new(pro)?;
            let soc = AsyncSocket::new(self.ctx.clone(), soc);
            match ops::async_connect(&soc, &ep, self.timeout).await {
                Ok(_) => {
                    return Ok(AsyncStreamSocket {
                        soc: soc,
                        timeout: self.timeout,
                        _marker: PhantomData,
                    });
                }
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }
}
