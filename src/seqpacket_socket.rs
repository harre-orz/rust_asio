use crate::IoContext;
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

pub struct AsyncSeqPacketSocket<P> {
    soc: AsyncSocket,
    _marker: PhantomData<P>,
}

impl<P> AsyncSeqPacketSocket<P>
where
    P: Protocol,
{
    pub const fn as_ctx(&self) -> &IoContext {
        self.soc.as_ctx()
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

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.as_socket().send(buf)
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

    pub async fn async_send(&self, buf: &[u8]) -> Result<usize> {
        ops::async_send(&self.soc, buf).await
    }
}

pub struct SeqPacketSocket<P> {
    blk: Blocking,
    soc: Socket,
    _marker: PhantomData<P>,
}

impl<P> SeqPacketSocket<P>
where
    P: Protocol,
{
    pub(crate) fn new_impl(ctx: IoContext, soc: Socket) -> Self {
        Self {
            blk: Blocking::new(ctx),
            soc: soc,
            _marker: PhantomData,
        }
    }

    pub const fn as_ctx(&self) -> &IoContext {
        &self.blk.as_ctx()
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

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.send(buf)
    }

    pub fn close(self) -> Result<()> {
        self.soc.close()
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        self.soc.shutdown(how)
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
}

/// Converts Asynchronous socket.
///
/// # Examples
///
/// ```no_run
/// use asyncio::IoContext;
/// use asyncio::local::{LocalSeqPacketEndpoint, LocalSeqPacketSocket, AsyncLocalSeqPacketSocket};
/// use std::path::Path;
///
/// let ctx = &IoContext::new().unwrap();
/// let ep = LocalSeqPacketEndpoint::new(Path::new("/foo/bar")).unwrap();
/// let soc = LocalSeqPacketSocket::new(ctx).connect(&ep).unwrap();
/// let soc = AsyncLocalSeqPacketSocket::from(soc);
/// ```
impl<P> From<SeqPacketSocket<P>> for AsyncSeqPacketSocket<P> {
    fn from(soc: SeqPacketSocket<P>) -> Self {
        Self {
            soc: AsyncSocket::new(soc.blk.into_ctx(), soc.soc),
            _marker: PhantomData,
        }
    }
}

pub struct SeqPacketSocketBuilder<P: Protocol> {
    ctx: IoContext,
    pro: P::Type,
}

impl<P: Protocol> SeqPacketSocketBuilder<P> {
    pub(crate) const fn new_impl(ctx: IoContext, pro: P::Type) -> Self {
        Self { ctx: ctx, pro: pro }
    }

    pub fn connect<'a, E>(self, eps: &'a E) -> Result<SeqPacketSocket<P>>
    where
        P: 'a,
        E: Endpoints<'a, P>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in eps.endpoints() {
            let pro = P::new(ep.sockaddr_ref().address_family(), self.pro);
            let soc = Socket::new(pro)?;
            match soc.connect(&ep) {
                Ok(_) => return Ok(SeqPacketSocket::new_impl(self.ctx, soc)),
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }
}
