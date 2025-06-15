use crate::IoContext;
use crate::error::OsError;
use crate::exec::async_socket::AsyncSocket;
use crate::ffi::socket::Socket;
use crate::ops;
use crate::socket_base::{MAX_CONNECTIONS, Protocol, Shutdown};
use std::cell::Cell;
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, OsError>;

pub trait ConnectedSocket {
    type Socket;

    fn socket(&self, soc: Socket) -> Self::Socket;
}

pub struct SocketListenerBuilder<'a, P: Protocol> {
    ctx: &'a IoContext,
    soc: Socket,
    pro: P,
    max_conns: i32,
}

impl<'a, P> SocketListenerBuilder<'a, P>
where
    P: Protocol,
{
    pub fn max_conns(mut self, max_conns: i32) -> Self {
        self.max_conns = max_conns;
        self
    }

    pub fn bind(self, ep: &P::Endpoint) -> Result<Self> {
        self.soc.bind(ep)?;
        Ok(self)
    }

    pub fn reuse_addr(self, on: bool) -> Result<Self> {
        self.soc.reuse_addr(on)?;
        Ok(self)
    }

    pub fn listen(self) -> Result<SocketListener<P>> {
        self.soc.listen(self.max_conns)?;
        Ok(SocketListener {
            ctx: self.ctx.clone(),
            soc: self.soc,
            pro: self.pro,
            cto: Cell::new(None),
        })
    }

    pub fn listen_async(self) -> Result<AsyncSocketListener<P>> {
        let soc = self.listen()?;
        Ok(soc.into())
    }
}

pub struct SocketListener<P> {
    ctx: IoContext,
    soc: Socket,
    pro: P,
    cto: Cell<Option<Instant>>,
}

impl<P> SocketListener<P>
where
    P: Protocol,
{
    pub fn new(ctx: &IoContext, pro: P) -> Result<SocketListenerBuilder<P>> {
        let soc = Socket::new(pro)?;
        Ok(SocketListenerBuilder {
            ctx,
            soc,
            pro,
            max_conns: MAX_CONNECTIONS,
        })
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn close(self) -> Result<()> {
        self.soc.close()
    }

    pub fn expires_at(&self, cto: Instant) {
        self.cto.set(Some(cto))
    }

    pub fn expires_from_now(&self, cto: Duration) {
        self.expires_at(Instant::now() + cto)
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.getsockname()
    }

    pub fn protocol(&self) -> P {
        self.pro
    }
}

impl<P> SocketListener<P>
where
    P: Protocol,
    Self: ConnectedSocket,
{
    pub fn nb_accept(&self) -> Result<(<Self as ConnectedSocket>::Socket, P::Endpoint)> {
        let (soc, ep) = self.soc.nb_accept()?;
        Ok((self.socket(soc), ep))
    }

    pub fn accept(&self) -> Result<(<Self as ConnectedSocket>::Socket, P::Endpoint)> {
        let (soc, ep) = ops::accept(&self.soc, &self.ctx, self.cto.replace(None))?;
        Ok((self.socket(soc), ep))
    }
}

pub struct AsyncSocketListener<P> {
    soc: AsyncSocket,
    pro: P,
}

impl<P> AsyncSocketListener<P>
where
    P: Protocol,
{
    pub fn as_ctx(&self) -> &IoContext {
        self.soc.as_ctx()
    }

    pub fn expires_at(&self, time: Instant) {
        self.soc.update_schedule(time)
    }

    pub fn expires_from_now(&self, time: Duration) {
        self.expires_at(Instant::now() + time)
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.as_socket().getsockname()
    }

    pub fn protocol(&self) -> P {
        self.pro
    }
}

impl<P> AsyncSocketListener<P>
where
    P: Protocol,
    Self: ConnectedSocket,
{
    pub async fn async_accept(&self) -> Result<(<Self as ConnectedSocket>::Socket, P::Endpoint)> {
        let (soc, ep) = ops::async_accept(&self.soc).await?;
        Ok((self.socket(soc), ep))
    }

    pub fn nb_accept(&self) -> Result<(<Self as ConnectedSocket>::Socket, P::Endpoint)> {
        let (soc, ep) = self.soc.as_socket().nb_accept()?;
        Ok((self.socket(soc), ep))
    }
}

impl<P> From<SocketListener<P>> for AsyncSocketListener<P> {
    fn from(soc: SocketListener<P>) -> Self {
        Self {
            soc: AsyncSocket::new(soc.ctx, soc.soc),
            pro: soc.pro,
        }
    }
}
