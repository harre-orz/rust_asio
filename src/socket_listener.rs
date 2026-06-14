use crate::core::IoContext;
use crate::error::{OsError, Result};
use crate::primitive::{Socket, Timeout};
use crate::socket::AsyncSocket;
use crate::socket_base::{EndpointRef, GetSockOpt, MAX_CONNECTIONS, Protocol, SetSockOpt};
use std::any::Any;
use std::collections::LinkedList;
use std::time::Duration;

pub trait ConnectedSocket<P>
where
    P: Protocol,
{
    type Socket;

    fn connected(&self, soc: Socket, pro: P) -> Self::Socket;
}

pub struct SocketListener<P>
where
    P: Protocol,
{
    ctx: IoContext,
    soc: Socket,
    pro: P,
    t: Timeout,
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
            t: Timeout::INFINITE,
        }
    }

    pub const fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn close(self) -> Result<()> {
        self.soc.close()
    }

    pub const fn set_timeout(&mut self, timeout: Duration) {
        self.t = Timeout::from_duration(timeout)
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

    pub const fn protocol(&self) -> P {
        self.pro
    }
}

impl<P> SocketListener<P>
where
    P: Protocol,
    Self: ConnectedSocket<P>,
{
    pub fn nb_accept(&self) -> Result<(<Self as ConnectedSocket<P>>::Socket, P::Endpoint)> {
        let (soc, ep) = self.soc.nb_accept()?;
        Ok((self.connected(soc, self.pro), ep))
    }

    pub fn accept(&self) -> Result<(<Self as ConnectedSocket<P>>::Socket, P::Endpoint)> {
        let (soc, ep) = self.soc.accept(&self.ctx, self.t)?;
        Ok((self.connected(soc, self.pro), ep))
    }
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
    soc: AsyncSocket<()>,
    pro: P,
    t: Timeout,
}

impl<P> AsyncSocketListener<P>
where
    P: Protocol,
{
    pub fn as_ctx(&self) -> &IoContext {
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
        self.t = Timeout::from_duration(timeout)
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.as_socket().getsockname()
    }

    pub const fn protocol(&self) -> P {
        self.pro
    }
}

impl<P> AsyncSocketListener<P>
where
    P: Protocol,
    Self: ConnectedSocket<P>,
{
    pub fn nb_accept(&self) -> Result<(<Self as ConnectedSocket<P>>::Socket, P::Endpoint)> {
        let (soc, ep) = self.soc.as_socket().nb_accept()?;
        Ok((self.connected(soc, self.pro), ep))
    }

    pub async fn async_accept(
        &self,
    ) -> Result<(<Self as ConnectedSocket<P>>::Socket, P::Endpoint)> {
        #[cfg(unix)]
        let (soc, ep) = self.soc.async_accept(self.t).await?;
        #[cfg(windows)]
        let (soc, ep) = socket::async_accept(&self.soc, self.timeout, self.pro).await?;
        Ok((self.connected(soc, self.pro), ep))
    }
}

impl<P> From<SocketListener<P>> for AsyncSocketListener<P>
where
    P: Protocol,
{
    fn from(soc: SocketListener<P>) -> Self {
        Self {
            soc: AsyncSocket::new(soc.ctx, soc.soc, ()),
            pro: soc.pro,
            t: soc.t,
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
    sock_opts: LinkedList<Box<dyn SetSockOpt<P> + 'static>>,
}

impl<P> SocketListenerBuilder<P>
where
    P: Protocol,
{
    pub(crate) const fn new_impl(ctx: IoContext, pro: P::Type) -> Self {
        Self {
            ctx: ctx,
            pro: pro,
            max_conns: MAX_CONNECTIONS as i32,
            sock_opts: LinkedList::new(),
        }
    }

    pub fn listen<'a, E>(self, eps: E) -> Result<SocketListener<P>>
    where
        E: IntoIterator<Item = EndpointRef<'a, <P as Protocol>::Endpoint>>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in eps {
            let pro = P::new(&ep, self.pro);
            let soc = Socket::new(pro)?;
            for opt in &self.sock_opts {
                soc.setsockopt(pro, opt.as_ref())?;
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

    pub fn set_option<T>(self, opt: T) -> Self
    where
        T: SetSockOpt<P>,
    {
        let opt: Box<dyn SetSockOpt<P>> = Box::new(opt);
        let mut sock_opts = LinkedList::new();
        for sock_opt in self.sock_opts {
            if opt.type_id() != sock_opt.type_id() {
                sock_opts.push_back(sock_opt);
            }
        }
        sock_opts.push_back(opt);
        Self {
            ctx: self.ctx,
            pro: self.pro,
            max_conns: self.max_conns,
            sock_opts: sock_opts,
        }
    }
}
