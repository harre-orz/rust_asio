use crate::IoContext;
use crate::buffer::{AsyncIoStream, IoStream};
use crate::error::OsError;
use crate::exec::AsyncSocket;
use crate::ops::{self, Blocking};
use crate::sockaddr::SockAddr;
use crate::socket::Socket;
use crate::socket_base::{Endpoints, Protocol, Shutdown};
use std::marker::PhantomData;
use std::result;
use std::time::{Duration, Instant};

type Result<T> = result::Result<T, OsError>;

pub struct AsyncStreamSocket<P> {
    soc: AsyncSocket,
    _marker: PhantomData<P>,
}

impl<P: Protocol> AsyncStreamSocket<P> {
    pub fn expires_at(&self, timeout: Instant) {
        self.soc.update_schedule(timeout);
    }

    pub fn expires_from_now(&self, timeout: Duration) {
        self.soc.update_schedule(Instant::now() + timeout);
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

pub struct StreamSocket<P> {
    blk: Blocking,
    soc: Socket,
    _marker: PhantomData<P>,
}

impl<P: Protocol> StreamSocket<P> {
    pub(crate) fn new_impl(ctx: IoContext, soc: Socket) -> Self {
        Self {
            blk: Blocking::new(ctx),
            soc: soc,
            _marker: PhantomData,
        }
    }

    pub fn close(self) -> Result<()> {
        self.soc.close()
    }

    pub fn expires_at(&self, timeout: Instant) {
        self.blk.expires_at(timeout)
    }

    pub fn expires_from_now(&self, timeout: Duration) {
        self.blk.expires_from_now(timeout)
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
        ops::read_some(&self.soc, buf, &self.blk)
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize> {
        ops::receive(&self.soc, buf, &self.blk)
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.getpeername()
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize> {
        ops::send(&self.soc, buf, &self.blk)
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        self.soc.shutdown(how)
    }

    pub fn write_some(&self, buf: &[u8]) -> Result<usize> {
        ops::write_some(&self.soc, buf, &self.blk)
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
impl<P> From<StreamSocket<P>> for AsyncStreamSocket<P> {
    fn from(soc: StreamSocket<P>) -> Self {
        Self {
            soc: AsyncSocket::new(soc.blk.into_ctx(), soc.soc),
            _marker: PhantomData,
        }
    }
}

pub struct StreamSocketBuilder<P: Protocol> {
    blk: Blocking,
    pro: P::Type,
}

impl<P: Protocol> StreamSocketBuilder<P> {
    pub(crate) fn new_impl(ctx: IoContext, pro: P::Type) -> Self {
        Self {
            blk: Blocking::new(ctx),
            pro: pro,
        }
    }

    pub fn nb_connect<'a, E>(self, eps: &'a E) -> Result<StreamSocket<P>>
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
                    let ctx = self.blk.into_ctx();
                    return Ok(StreamSocket::new_impl(ctx, soc));
                }
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }

    pub fn connect<'a, E>(self, eps: &'a E) -> Result<StreamSocket<P>>
    where
        P: 'a,
        E: Endpoints<'a, P>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in eps.endpoints() {
            let pro = P::new(ep.sockaddr_ref().address_family(), self.pro);
            let soc = Socket::new(pro)?;
            match ops::connect(&soc, &ep, &self.blk) {
                Ok(_) => {
                    let ctx = self.blk.into_ctx();
                    return Ok(StreamSocket::new_impl(ctx, soc));
                }
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }

    pub async fn async_connect<'a, E>(self, eps: &'a E) -> Result<AsyncStreamSocket<P>>
    where
        P: 'a,
        E: Endpoints<'a, P>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in eps.endpoints() {
            let pro = P::new(ep.sockaddr_ref().address_family(), self.pro);
            let soc = Socket::new(pro)?;
            let soc = AsyncSocket::new(self.blk.as_ctx().clone(), soc);
            match ops::async_connect(&soc, &ep).await {
                Ok(_) => {
                    return Ok(AsyncStreamSocket {
                        soc: soc,
                        _marker: PhantomData,
                    });
                }
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }
}
