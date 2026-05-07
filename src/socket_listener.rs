use crate::IoContext;
use crate::error::OsError;
use crate::exec::AsyncSocket;
use crate::ops::{self, Blocking};
use crate::sockaddr::SockAddr;
use crate::socket::Socket;
use crate::socket_base::{Endpoints, MAX_CONNECTIONS, Protocol, ReuseAddr};
use std::marker::PhantomData;
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, OsError>;

pub trait ConnectedSocket {
    type Socket;

    fn connected(&self, soc: Socket) -> Self::Socket;
}

/// ```no_run
/// use asyncio::IoContext;
/// use asyncio::local::{LocalStreamEndpoint, LocalStreamListener, AsyncLocalStreamListener};
/// use std::path::Path;
///
/// let ctx = &IoContext::new().unwrap();
/// let ep = LocalStreamEndpoint::new(Path::new("/foo/bar")).unwrap();
/// let soc = LocalStreamListener::new(ctx).listen(&ep).unwrap();
/// let soc = AsyncLocalStreamListener::from(soc);
/// ```
pub struct AsyncSocketListener<P> {
    soc: AsyncSocket,
    _marker: PhantomData<P>,
}

impl<P> AsyncSocketListener<P>
where
    P: Protocol,
{
    pub const fn as_ctx(&self) -> &IoContext {
        self.soc.as_ctx()
    }

    pub fn expires_at(&self, timeout: Instant) {
        self.soc.update_schedule(timeout);
    }

    pub fn expires_from_now(&self, timeout: Duration) {
        self.soc.update_schedule(Instant::now() + timeout);
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.as_socket().getsockname()
    }
}

impl<P> AsyncSocketListener<P>
where
    P: Protocol,
    Self: ConnectedSocket,
{
    pub fn nb_accept(&self) -> Result<(<Self as ConnectedSocket>::Socket, P::Endpoint)> {
        let (soc, ep) = self.soc.as_socket().accept()?;
        Ok((self.connected(soc), ep))
    }

    pub async fn async_accept(&self) -> Result<(<Self as ConnectedSocket>::Socket, P::Endpoint)> {
        let (soc, ep) = ops::async_accept(&self.soc).await?;
        Ok((self.connected(soc), ep))
    }
}

pub struct SocketListener<P> {
    blk: Blocking,
    soc: Socket,
    _marker: PhantomData<P>,
}

impl<P> SocketListener<P>
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

    pub fn close(self) -> Result<()> {
        self.soc.close()
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
}

impl<P> SocketListener<P>
where
    P: Protocol,
    Self: ConnectedSocket,
{
    pub fn accept(&self) -> Result<(<Self as ConnectedSocket>::Socket, P::Endpoint)> {
        let (soc, ep) = ops::accept(&self.soc, &self.blk)?;
        Ok((self.connected(soc), ep))
    }

    pub fn nb_accept(&self) -> Result<(<Self as ConnectedSocket>::Socket, P::Endpoint)> {
        let (soc, ep) = self.soc.accept()?;
        Ok((self.connected(soc), ep))
    }
}

impl<P> From<SocketListener<P>> for AsyncSocketListener<P> {
    fn from(soc: SocketListener<P>) -> Self {
        Self {
            soc: AsyncSocket::new(soc.blk.into_ctx(), soc.soc),
            _marker: PhantomData,
        }
    }
}

pub struct SocketListenerBuilder<P>
where
    P: Protocol,
{
    ctx: IoContext,
    pro: P::Type,
    reuse_addr: bool,
    max_conns: i32,
}

impl<P> SocketListenerBuilder<P>
where
    P: Protocol,
{
    pub(crate) const fn new_impl(ctx: IoContext, pro: P::Type) -> Self {
        Self {
            ctx: ctx,
            pro: pro,
            reuse_addr: false,
            max_conns: MAX_CONNECTIONS,
        }
    }

    pub fn listen<'a, E>(self, eps: &'a E) -> Result<SocketListener<P>>
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
                Ok(_) => {
                    soc.listen(self.max_conns)?;
                    return Ok(SocketListener::new_impl(self.ctx, soc));
                }
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }

    pub const fn reuse_addr(mut self, reuse_addr: bool) -> Self {
        self.reuse_addr = reuse_addr;
        self
    }

    pub const fn max_conns(mut self, max_conns: i32) -> Self {
        self.max_conns = max_conns;
        self
    }
}
