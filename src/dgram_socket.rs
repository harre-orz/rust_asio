use crate::buffer::MsgBuf;
use crate::core::IoContext;
use crate::error::{OsError, Result};
use crate::primitive::{Socket, Timeout};
use crate::socket::AsyncSocket;
use crate::socket_base::{EndpointRef, GetSockOpt, Protocol, SetSockOpt, Shutdown};
use std::any::Any;
use std::collections::LinkedList;
use std::time::Duration;

pub struct DgramSocket<P>
where
    P: Protocol,
{
    ctx: IoContext,
    soc: Socket,
    pro: P,
    t: Timeout,
}

impl<P> DgramSocket<P>
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

    pub fn bind<'a, E>(&self, it: E) -> Result<()>
    where
        E: IntoIterator<Item = EndpointRef<'a, <P as Protocol>::Endpoint>>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in it {
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

    pub fn connect<'a, E>(&self, it: E) -> Result<()>
    where
        E: IntoIterator<Item = EndpointRef<'a, <P as Protocol>::Endpoint>>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in it {
            match self.soc.nb_connect(&ep) {
                Ok(()) => return Ok(()),
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }

    pub const fn set_timeout(&mut self, timeout: Duration) {
        self.t = Timeout::from_duration(timeout)
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
        self.soc.nb_recv(buf)
    }

    pub fn nb_receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint)> {
        let (len, ep) = self.soc.nb_recvfrom(buf)?;
        Ok((len, ep))
    }

    pub fn nb_receive_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.soc.nb_recvmsg(mbuf, &self.ctx)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.nb_send(buf)
    }

    pub fn nb_send_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.soc.nb_sendmsg(mbuf)
    }

    pub fn nb_send_to(&self, buf: &[u8], ep: &P::Endpoint) -> Result<usize> {
        self.soc.nb_sendto(buf, &EndpointRef::new(ep))
    }

    pub const fn protocol(&self) -> P {
        self.pro
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.recv(&self.ctx, buf, self.t)
    }

    pub fn receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint)> {
        self.soc.recvfrom(&self.ctx, buf, self.t)
    }

    pub fn receive_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.soc.recvmsg(&self.ctx, mbuf, self.t)
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
        self.soc.send(&self.ctx, &buf, self.t)
    }

    pub fn send_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.soc.sendmsg(&self.ctx, mbuf, self.t)
    }

    pub fn send_to(&self, buf: &[u8], ep: &P::Endpoint) -> Result<usize> {
        self.soc
            .sendto(&self.ctx, buf, &EndpointRef::new(ep), self.t)
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
    t: Timeout,
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
        E: IntoIterator<Item = EndpointRef<'a, <P as Protocol>::Endpoint>>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in eps {
            match self.soc.as_socket().bind(&ep) {
                Ok(()) => return Ok(()),
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }

    pub fn connect<'a, E>(&self, it: E) -> Result<()>
    where
        E: IntoIterator<Item = EndpointRef<'a, <P as Protocol>::Endpoint>>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in it {
            match self.soc.as_socket().nb_connect(&ep) {
                Ok(()) => return Ok(()),
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }

    pub const fn set_timeout(&mut self, timeout: Duration) {
        self.t = Timeout::from_duration(timeout)
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
        self.soc.as_socket().nb_recv(buf)
    }

    pub fn nb_receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint)> {
        self.soc.as_socket().nb_recvfrom(buf)
    }

    pub fn nb_receive_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.soc.as_socket().nb_recvmsg(mbuf, self.as_ctx())
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.as_socket().nb_send(buf)
    }

    pub fn nb_send_to(&self, buf: &[u8], ep: &P::Endpoint) -> Result<usize> {
        self.soc.as_socket().nb_sendto(buf, &EndpointRef::new(ep))
    }

    pub fn nb_send_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.soc.as_socket().nb_sendmsg(mbuf)
    }

    pub const fn protocol(&self) -> P {
        self.pro
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.as_socket().recv(self.soc.as_ctx(), buf, self.t)
    }

    pub fn receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint)> {
        self.soc
            .as_socket()
            .recvfrom(self.soc.as_ctx(), buf, self.t)
    }

    pub fn receive_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.soc
            .as_socket()
            .recvmsg(self.soc.as_ctx(), mbuf, self.t)
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.as_socket().getpeername()
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.as_socket().send(self.as_ctx(), buf, self.t)
    }

    pub fn send_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.soc.as_socket().sendmsg(self.as_ctx(), mbuf, self.t)
    }

    pub fn send_to(&self, buf: &[u8], ep: &P::Endpoint) -> Result<usize> {
        self.soc
            .as_socket()
            .sendto(self.as_ctx(), buf, &EndpointRef::new(ep), self.t)
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
        self.soc.async_recv(buf, self.t).await
    }

    pub async fn async_receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint)> {
        self.soc.async_recvfrom(buf, self.t).await
    }

    pub async fn async_receive_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.soc.async_recvmsg(mbuf, self.t).await
    }

    pub async fn async_send(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.async_send(buf, self.t).await
    }

    pub async fn async_send_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.soc.async_sendmsg(mbuf, self.t).await
    }

    pub async fn async_send_to(&self, buf: &mut [u8], ep: &P::Endpoint) -> Result<usize> {
        self.soc
            .async_sendto(buf, &EndpointRef::new(ep), self.t)
            .await
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
            soc: AsyncSocket::new(soc.ctx, soc.soc),
            pro: soc.pro,
            t: soc.t,
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
        let soc = Socket::new(pro)?;
        Ok(DgramSocket {
            ctx: self.ctx,
            soc: soc,
            pro: pro,
            t: Timeout::INFINITE,
        })
    }

    pub fn bind<'a, E>(self, it: E) -> Result<DgramSocket<P>>
    where
        E: IntoIterator<Item = EndpointRef<'a, <P as Protocol>::Endpoint>>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in it {
            let pro = P::new(&ep, self.pro);
            let soc = Socket::new(pro)?;
            for opt in &self.sock_opts {
                soc.setsockopt(pro, opt.as_ref())?;
            }
            match soc.bind(&ep) {
                Ok(_) => return Ok(DgramSocket::new_impl(self.ctx, soc, pro)),
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
