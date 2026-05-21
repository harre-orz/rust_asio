use crate::IoContext;
use crate::error::{OsError, Result};
use crate::exec::AsyncSocket;
use crate::ops;
use crate::socket::{MAX_CONNECTIONS, Socket, Timeout};
use crate::socket_base::{Endpoints, GetSockOpt, Protocol, ReuseAddr, ReusePort, SetSockOpt};
use std::time::Duration;

pub trait ConnectedSocket<P>
where
    P: Protocol,
{
    type Socket;

    fn connected(&self, soc: Socket, pro: P) -> Self::Socket;
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
pub struct AsyncSocketListener<P>
where
    P: Protocol,
{
    soc: AsyncSocket,
    pro: P,
    timeout: Timeout,
}

impl<P> AsyncSocketListener<P>
where
    P: Protocol,
{
    pub const fn as_ctx(&self) -> &IoContext {
        self.soc.as_ctx()
    }

    pub fn get_option<T>(&self) -> Result<T>
    where
        T: GetSockOpt<P>,
    {
        self.soc.as_socket().getsockopt(self.pro)
    }

    pub fn set_option<T>(&self, opt: &T) -> Result<()>
    where
        T: SetSockOpt<P>,
    {
        self.soc.as_socket().setsockopt(self.pro, opt)
    }

    pub const fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = Timeout::from_duration(timeout)
    }
    pub fn local_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.as_socket().getsockname()
    }
}

impl<P> AsyncSocketListener<P>
where
    P: Protocol,
    Self: ConnectedSocket<P>,
{
    pub fn nb_accept(&self) -> Result<(<Self as ConnectedSocket<P>>::Socket, P::Endpoint)> {
        let (soc, ep) = self.soc.as_socket().accept()?;
        Ok((self.connected(soc, self.pro), ep))
    }

    pub async fn async_accept(
        &self,
    ) -> Result<(<Self as ConnectedSocket<P>>::Socket, P::Endpoint)> {
        let (soc, ep) = ops::async_accept(&self.soc, self.timeout).await?;
        Ok((self.connected(soc, self.pro), ep))
    }
}

pub struct SocketListener<P>
where
    P: Protocol,
{
    ctx: IoContext,
    soc: Socket,
    pro: P,
    timeout: Timeout,
}

impl<P> SocketListener<P>
where
    P: Protocol,
{
    pub(crate) const fn new_impl(ctx: IoContext, soc: Socket, pro: P) -> Self {
        Self {
            ctx: ctx,
            soc: soc,
            pro: pro,
            timeout: Timeout::infinite(),
        }
    }

    pub const fn as_ctx(&self) -> &IoContext {
        &self.ctx
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

    pub fn get_option<T>(&self) -> Result<T>
    where
        T: GetSockOpt<P>,
    {
        self.soc.getsockopt(self.pro)
    }

    pub fn set_option<T>(&self, opt: &T) -> Result<()>
    where
        T: SetSockOpt<P>,
    {
        self.soc.setsockopt(self.pro, opt)
    }
}

impl<P> SocketListener<P>
where
    P: Protocol,
    Self: ConnectedSocket<P>,
{
    pub fn accept(&self) -> Result<(<Self as ConnectedSocket<P>>::Socket, P::Endpoint)> {
        let (soc, ep) = ops::accept(&self.ctx, &self.soc, self.timeout)?;
        Ok((self.connected(soc, self.pro), ep))
    }

    pub fn nb_accept(&self) -> Result<(<Self as ConnectedSocket<P>>::Socket, P::Endpoint)> {
        let (soc, ep) = self.soc.accept()?;
        Ok((self.connected(soc, self.pro), ep))
    }
}

impl<P> From<SocketListener<P>> for AsyncSocketListener<P>
where
    P: Protocol,
{
    fn from(soc: SocketListener<P>) -> Self {
        Self {
            soc: AsyncSocket::new(soc.ctx, soc.soc),
            pro: soc.pro,
            timeout: soc.timeout,
        }
    }
}

pub struct SocketListenerBuilder<P>
where
    P: Protocol,
{
    ctx: IoContext,
    pro: P::Type,
    max_conns: i32,
    reuse_addr: bool,
    reuse_port: bool,
}

impl<P> SocketListenerBuilder<P>
where
    P: Protocol,
{
    pub(crate) const fn new_impl(ctx: IoContext, pro: P::Type) -> Self {
        Self {
            ctx: ctx,
            pro: pro,
            max_conns: MAX_CONNECTIONS,
            reuse_addr: false,
            reuse_port: false,
        }
    }

    pub fn listen<'a, E>(self, eps: E) -> Result<SocketListener<P>>
    where
        P: 'a,
        E: Endpoints<'a, P>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in eps.endpoints() {
            let pro = P::new(&ep, self.pro);
            let soc = Socket::new(pro)?;
            if self.reuse_addr {
                soc.setsockopt(pro, &ReuseAddr::ON)?;
            }
            if self.reuse_port {
                soc.setsockopt(pro, &ReusePort::ON)?;
            }
            match soc.bind(&ep) {
                Ok(_) => {
                    soc.listen(self.max_conns)?;
                    return Ok(SocketListener::new_impl(self.ctx, soc, pro));
                }
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }

    pub fn protocol_type(&self) -> P::Type {
        self.pro
    }

    pub const fn max_conns(mut self, max_conns: i32) -> Self {
        self.max_conns = max_conns;
        self
    }

    pub const fn reuse_addr(mut self, on: bool) -> Self {
        self.reuse_addr = on;
        self
    }

    pub const fn reuse_port(mut self, on: bool) -> Self {
        self.reuse_port = on;
        self
    }
}
