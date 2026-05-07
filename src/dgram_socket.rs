use crate::IoContext;
use crate::buffer::MsgBuf;
use crate::error::OsError;
use crate::exec::AsyncSocket;
use crate::ops::{self, Blocking};
use crate::sockaddr::{AddressFamily, SockAddr};
use crate::socket::Socket;
use crate::socket_base::{EndpointRef, Endpoints, Protocol, ReuseAddr, Shutdown};
use std::marker::PhantomData;
use std::result;
use std::time::{Duration, Instant};

type Result<T> = result::Result<T, OsError>;

pub struct AsyncDgramSocket<P> {
    soc: AsyncSocket,
    _marker: PhantomData<P>,
}

impl<P> AsyncDgramSocket<P>
where
    P: Protocol,
{
    pub fn as_ctx(&self) -> &IoContext {
        &self.soc.as_ctx()
    }

    pub fn bind<'a, E>(&self, eps: &'a E) -> Result<()>
    where
        P: 'a,
        E: Endpoints<'a, P>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in eps.endpoints() {
            match self.soc.as_socket().bind(&ep) {
                Ok(()) => return Ok(()),
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }

    pub fn connect<'a, E>(&self, eps: &'a E) -> Result<()>
    where
        P: 'a,
        E: Endpoints<'a, P>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in eps.endpoints() {
            match self.soc.as_socket().connect(&ep) {
                Ok(()) => return Ok(()),
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }

    pub fn expires_at(&self, timeout: Instant) {
        self.soc.update_schedule(timeout)
    }

    pub fn expires_from_now(&self, timeout: Duration) {
        self.expires_at(Instant::now() + timeout)
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.as_socket().getsockname()
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.as_socket().receive(buf)
    }

    pub fn nb_receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint)> {
        self.soc.as_socket().receive_from(buf)
    }

    pub fn nb_receive_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.soc.as_socket().receive_msg(mbuf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.as_socket().send(buf)
    }

    pub fn nb_send_to(&self, buf: &[u8], ep: &P::Endpoint) -> Result<usize> {
        self.soc.as_socket().send_to(buf, &EndpointRef::new(ep))
    }

    pub fn nb_send_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.soc.as_socket().send_msg(mbuf)
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.as_socket().getpeername()
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        self.soc.as_socket().shutdown(how)
    }

    pub async fn async_receive(&self, buf: &mut [u8]) -> Result<usize> {
        ops::async_receive(&self.soc, buf).await
    }

    pub async fn async_receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint)> {
        ops::async_receive_from(&self.soc, buf).await
    }

    pub async fn async_receive_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        ops::async_receive_msg(&self.soc, mbuf).await
    }

    pub async fn async_send(&self, buf: &mut [u8]) -> Result<usize> {
        ops::async_send(&self.soc, buf).await
    }

    pub async fn async_send_to(&self, buf: &mut [u8], ep: &P::Endpoint) -> Result<usize> {
        ops::async_send_to(&self.soc, buf, &EndpointRef::new(ep)).await
    }

    pub async fn async_send_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        ops::async_send_msg(&self.soc, mbuf).await
    }
}

pub struct DgramSocket<P> {
    blk: Blocking,
    soc: Socket,
    _marker: PhantomData<P>,
}

impl<P> DgramSocket<P>
where
    P: Protocol,
{
    pub(crate) const fn new_impl(ctx: IoContext, soc: Socket) -> Self {
        Self {
            blk: Blocking::new(ctx),
            soc: soc,
            _marker: PhantomData,
        }
    }

    pub const fn as_ctx(&self) -> &IoContext {
        &self.blk.as_ctx()
    }

    pub fn bind<'a, E>(&self, eps: &'a E) -> Result<()>
    where
        P: 'a,
        E: Endpoints<'a, P>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in eps.endpoints() {
            match self.soc.bind(&ep) {
                Ok(()) => return Ok(()),
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }

    pub fn close(self) -> Result<()> {
        self.soc.close()
    }

    pub fn connect<'a, E>(&self, eps: &'a E) -> Result<()>
    where
        P: 'a,
        E: Endpoints<'a, P>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in eps.endpoints() {
            match self.soc.connect(&ep) {
                Ok(()) => return Ok(()),
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }

    pub fn expires_at(&self, time: Instant) {
        self.blk.expires_at(time)
    }

    pub fn expires_from_now(&self, time: Duration) {
        self.blk.expires_from_now(time)
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.getsockname()
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.receive(buf)
    }

    pub fn nb_receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint)> {
        let (len, ep) = self.soc.receive_from(buf)?;
        Ok((len, ep))
    }

    pub fn nb_receive_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.soc.receive_msg(mbuf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.send(buf)
    }

    pub fn nb_send_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.soc.send_msg(mbuf)
    }

    pub fn nb_send_to(&self, buf: &[u8], ep: &P::Endpoint) -> Result<usize> {
        self.soc.send_to(buf, &EndpointRef::new(ep))
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize> {
        ops::receive(&self.soc, buf, &self.blk)
    }

    pub fn receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint)> {
        ops::receive_from(&self.soc, buf, &self.blk)
    }

    pub fn receive_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        ops::receive_msg(&self.soc, mbuf, &self.blk)
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.getpeername()
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize> {
        ops::send(&self.soc, buf, &self.blk)
    }

    pub fn send_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        ops::send_msg(&self.soc, mbuf, &self.blk)
    }

    pub fn send_to(&self, buf: &[u8], ep: &P::Endpoint) -> Result<usize> {
        ops::send_to(&self.soc, buf, &EndpointRef::new(ep), &self.blk)
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        self.soc.shutdown(how)
    }
}

/// Converts Asynchronous socket.
///
/// # Examples
///
/// ```no_run
/// use asyncio::IoContext;
/// use asyncio::local::{LocalDgramEndpoint, LocalDgramSocket, AsyncLocalDgramSocket};
/// use std::path::Path;
/// use asyncio::sockaddr::AddressFamily;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = LocalDgramSocket::new(ctx).unbound(AddressFamily::AF_LOCAL).unwrap();
/// let soc = AsyncLocalDgramSocket::from(soc);
/// ```
impl<P> From<DgramSocket<P>> for AsyncDgramSocket<P> {
    fn from(soc: DgramSocket<P>) -> Self {
        Self {
            soc: AsyncSocket::new(soc.blk.into_ctx(), soc.soc),
            _marker: PhantomData,
        }
    }
}

pub struct DgramSocketBuilder<P>
where
    P: Protocol,
{
    ctx: IoContext,
    pro: P::Type,
    reuse_addr: bool,
}

impl<P> DgramSocketBuilder<P>
where
    P: Protocol,
{
    pub(crate) const fn new_impl(ctx: IoContext, pro: P::Type) -> Self {
        DgramSocketBuilder {
            ctx: ctx,
            pro: pro,
            reuse_addr: false,
        }
    }

    pub fn bind<'a, E>(self, eps: &'a E) -> Result<DgramSocket<P>>
    where
        P: 'a,
        E: Endpoints<'a, P>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in eps.endpoints() {
            let pro = P::new(ep.sockaddr_ref().address_family(), self.pro);
            let soc = Socket::new(pro)?;
            if self.reuse_addr {
                soc.setsockopt(&ReuseAddr::ON)?;
            }
            match soc.bind(&ep) {
                Ok(_) => return Ok(DgramSocket::new_impl(self.ctx, soc)),
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }

    pub fn unbound(self, address_family: AddressFamily) -> Result<DgramSocket<P>> {
        let pro = P::new(address_family, self.pro);
        let soc = Socket::new(pro)?;
        Ok(DgramSocket {
            blk: Blocking::new(self.ctx),
            soc: soc,
            _marker: PhantomData,
        })
    }

    pub fn reuse_addr(mut self, on: bool) -> Self {
        self.reuse_addr = on;
        self
    }
}
