use crate::IoContext;
use crate::buffer::MsgBuf;
use crate::poll::Timeout;
use crate::error::{OsError, Result};
use crate::socket::{AsyncSocket, Socket};
use crate::socket_base::{EndpointRef, Endpoints, GetSockOpt, Protocol, SetSockOpt, Shutdown};
use std::any::Any;
use std::collections::LinkedList;
use std::time::Duration;

pub struct DgramSocket<P>
where
    P: Protocol,
{
    soc: Socket,
    pro: P,
    timeout: Timeout,
}

impl<P> DgramSocket<P>
where
    P: Protocol,
{
    pub(crate) const fn new_impl(soc: Socket, pro: P) -> Self {
        Self {
            soc: soc,
            pro: pro,
            timeout: Timeout::infinite(),
        }
    }

    pub const fn as_ctx(&self) -> &IoContext {
        self.soc.as_ctx()
    }

    pub fn bind<'a, E>(&self, eps: E) -> Result<()>
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

    pub fn connect<'a, E>(&self, eps: E) -> Result<()>
    where
        P: 'a,
        E: Endpoints<'a, P>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in eps.endpoints() {
            match self.soc.nb_connect(&ep) {
                Ok(()) => return Ok(()),
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }

    pub const fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = Timeout::from_duration(timeout)
    }

    pub fn get_option<T>(&self) -> Result<T>
    where
        T: GetSockOpt<P>,
    {
        self.soc.getsockopt(self.pro)
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.getsockname()
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.nb_receive(buf)
    }

    pub fn nb_receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint)> {
        let (len, ep) = self.soc.nb_receive_from(buf)?;
        Ok((len, ep))
    }

    pub fn nb_receive_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        #[cfg(unix)]
        let res = self.soc.nb_receive_msg(mbuf);
        #[cfg(windows)]
        let res = self.soc.receive_msg(mbuf, &self.ctx);
        res
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.nb_send(buf)
    }

    pub fn nb_send_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.soc.nb_send_msg(mbuf)
    }

    pub fn nb_send_to(&self, buf: &[u8], ep: &P::Endpoint) -> Result<usize> {
        self.soc.nb_send_to(buf, &EndpointRef::new(ep))
    }

    pub const fn protocol(&self) -> P {
        self.pro
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.receive(buf, self.timeout)
    }

    pub fn receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint)> {
        self.soc.receive_from(buf, self.timeout)
    }

    pub fn receive_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.soc.receive_msg(mbuf, self.timeout)
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.getpeername()
    }

    pub fn set_option<T>(&self, opt: &T) -> Result<()>
    where
        T: SetSockOpt<P>,
    {
        self.soc.setsockopt(self.pro, opt)
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.send(&buf, self.timeout)
    }

    pub fn send_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.soc.send_msg(mbuf, self.timeout)
    }

    pub fn send_to(&self, buf: &[u8], ep: &P::Endpoint) -> Result<usize> {
        self.soc.send_to(buf, &EndpointRef::new(ep), self.timeout)
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        self.soc.shutdown(how)
    }
}


pub struct AsyncDgramSocket<P>
where
    P: Protocol,
{
    soc: AsyncSocket,
    pro: P,
    timeout: Timeout,
}

impl<P> AsyncDgramSocket<P>
where
    P: Protocol,
{
    pub fn as_ctx(&self) -> &IoContext {
        &self.soc.as_ctx()
    }

    pub fn bind<'a, E>(&self, eps: E) -> Result<()>
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

    pub fn connect<'a, E>(&self, eps: E) -> Result<()>
    where
        P: 'a,
        E: Endpoints<'a, P>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in eps.endpoints() {
            match self.soc.as_socket().nb_connect(&ep) {
                Ok(()) => return Ok(()),
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }

    pub const fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = Timeout::from_duration(timeout)
    }

    pub fn get_option<T>(&self) -> Result<T>
    where
        T: GetSockOpt<P>,
    {
        self.soc.as_socket().getsockopt(self.pro)
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.as_socket().getsockname()
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.as_socket().nb_receive(buf)
    }

    pub fn nb_receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint)> {
        self.soc.as_socket().nb_receive_from(buf)
    }

    pub fn nb_receive_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.soc.as_socket().nb_receive_msg(mbuf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.as_socket().nb_send(buf)
    }

    pub fn nb_send_to(&self, buf: &[u8], ep: &P::Endpoint) -> Result<usize> {
        self.soc.as_socket().nb_send_to(buf, &EndpointRef::new(ep))
    }

    pub fn nb_send_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.soc.as_socket().nb_send_msg(mbuf)
    }

    pub const fn protocol(&self) -> P {
        self.pro
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.as_socket().getpeername()
    }

    pub fn set_option<T>(&self, opt: &T) -> Result<()>
    where
        T: SetSockOpt<P>,
    {
        self.soc.as_socket().setsockopt(self.pro, opt)
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        self.soc.as_socket().shutdown(how)
    }

    pub async fn async_receive(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.async_receive(buf, self.timeout).await
    }

    pub async fn async_receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint)> {
        self.soc.async_receive_from(buf, self.timeout).await
    }

    pub async fn async_receive_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.soc.async_receive_msg(mbuf, self.timeout).await
    }

    pub async fn async_send(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.async_send(buf, self.timeout).await
    }

    pub async fn async_send_to(&self, buf: &mut [u8], ep: &P::Endpoint) -> Result<usize> {
        self.soc
            .async_send_to(buf, &EndpointRef::new(ep), self.timeout)
            .await
    }

    pub async fn async_send_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.soc.async_send_msg(mbuf, self.timeout).await
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
/// let soc = LocalDgramSocket::new(ctx).unbound().unwrap();
/// let soc = AsyncLocalDgramSocket::from(soc);
/// ```
impl<P> From<DgramSocket<P>> for AsyncDgramSocket<P>
where
    P: Protocol,
{
    fn from(soc: DgramSocket<P>) -> Self {
        Self {
            soc: AsyncSocket::new(soc.soc),
            pro: soc.pro,
            timeout: soc.timeout,
        }
    }
}

pub struct DgramSocketBuilder<P>
where
    P: Protocol,
{
    ctx: IoContext,
    pro: P::Type,
    sock_opts: LinkedList<Box<dyn SetSockOpt<P> + 'static>>,
}

impl<P> DgramSocketBuilder<P>
where
    P: Protocol,
{
    pub(crate) const fn new_impl(ctx: IoContext, pro: P::Type) -> Self {
        DgramSocketBuilder {
            ctx: ctx,
            pro: pro,
            sock_opts: LinkedList::new(),
        }
    }

    pub(crate) fn unbound_impl(self, pro: P) -> Result<DgramSocket<P>> {
        let soc = Socket::new(&self.ctx, pro)?;
        Ok(DgramSocket {
            soc: soc,
            pro: pro,
            timeout: Timeout::infinite(),
        })
    }

    pub fn bind<'a, E>(self, eps: E) -> Result<DgramSocket<P>>
    where
        P: 'a,
        E: Endpoints<'a, P>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in eps.endpoints() {
            let pro = P::new(&ep, self.pro);
            let soc = Socket::new(&self.ctx, pro)?;
            for opt in &self.sock_opts {
                soc.setsockopt(pro, opt.as_ref())?;
            }
            match soc.bind(&ep) {
                Ok(_) => return Ok(DgramSocket::new_impl(soc, pro)),
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }

    pub fn protocol_type(&self) -> P::Type {
        self.pro
    }

    pub fn set_option<T>(self, opt: T) -> Self
    where
        P: 'static,
        T: SetSockOpt<P> + 'static,
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
            sock_opts: sock_opts,
        }
    }
}
