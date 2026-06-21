use crate::buffer::MsgBuf;
use crate::core::IoContext;
use crate::error::OsError;
use crate::primitive::{AtomicTimeout, Socket, TimeoutError};
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
    ato: AtomicTimeout,
    pro: P,
}

impl<P> DgramSocket<P>
where
    P: Protocol,
{
    pub(crate) fn new_impl(ctx: IoContext, soc: Socket, pro: P) -> Self {
        let ato = ctx.timeout();
        Self {
            ctx: ctx,
            soc: soc,
            ato: ato,
            pro: pro,
        }
    }

    pub const fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn bind<'a, E>(&self, it: E) -> Result<(), OsError>
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

    pub fn close(self) -> Result<(), OsError> {
        self.soc.close()
    }

    pub fn connect<'a, E>(&self, it: E) -> Result<(), OsError>
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

    pub fn set_timeout(&self, timeout: Duration) -> Result<(), TimeoutError> {
        self.ato.set(timeout)
    }

    pub fn get_option<T>(&self) -> Result<T, OsError>
    where
        T: GetSockOpt<P>,
    {
        self.soc.getsockopt(self.pro)
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint, OsError> {
        self.soc.getsockname()
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.soc.nb_recv(buf)
    }

    pub fn nb_receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint), OsError> {
        let (len, ep) = self.soc.nb_recvfrom(buf)?;
        Ok((len, ep))
    }

    pub fn nb_receive_msg(&self, mbuf: &mut MsgBuf) -> Result<usize, OsError> {
        self.soc.nb_recvmsg(mbuf, &self.ctx)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.soc.nb_send(buf)
    }

    pub fn nb_send_msg(&self, mbuf: &mut MsgBuf) -> Result<usize, OsError> {
        self.soc.nb_sendmsg(mbuf)
    }

    pub fn nb_send_to(&self, buf: &[u8], ep: &P::Endpoint) -> Result<usize, OsError> {
        self.soc.nb_sendto(buf, &EndpointRef::new(ep))
    }

    pub const fn protocol(&self) -> P {
        self.pro
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.soc.recv(&self.ctx, buf, self.ato.get())
    }

    pub fn receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint), OsError> {
        self.soc.recvfrom(&self.ctx, buf, self.ato.get())
    }

    pub fn receive_msg(&self, mbuf: &mut MsgBuf) -> Result<usize, OsError> {
        self.soc.recvmsg(&self.ctx, mbuf, self.ato.get())
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint, OsError> {
        self.soc.getpeername()
    }

    pub fn set_option<T>(&self, opt: &T) -> Result<(), OsError>
    where
        T: SetSockOpt<P>,
    {
        self.soc.setsockopt(self.pro, opt)
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.soc.send(&self.ctx, &buf, self.ato.get())
    }

    pub fn send_msg(&self, mbuf: &mut MsgBuf) -> Result<usize, OsError> {
        self.soc.sendmsg(&self.ctx, mbuf, self.ato.get())
    }

    pub fn send_to(&self, buf: &[u8], ep: &P::Endpoint) -> Result<usize, OsError> {
        self.soc
            .sendto(&self.ctx, buf, &EndpointRef::new(ep), self.ato.get())
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<(), OsError> {
        self.soc.shutdown(how)
    }
}

pub struct AsyncDgramSocket<P>
where
    P: Protocol,
{
    inner: AsyncSocket<P>,
}

impl<P> AsyncDgramSocket<P>
where
    P: Protocol,
{
    pub fn as_ctx(&self) -> &IoContext {
        &self.inner.0.1.0
    }

    pub fn bind<'a, E>(&self, eps: E) -> Result<(), OsError>
    where
        E: IntoIterator<Item = EndpointRef<'a, <P as Protocol>::Endpoint>>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in eps {
            match self.inner.0.1.1.bind(&ep) {
                Ok(()) => return Ok(()),
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }

    pub fn connect<'a, E>(&self, it: E) -> Result<(), OsError>
    where
        E: IntoIterator<Item = EndpointRef<'a, <P as Protocol>::Endpoint>>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in it {
            match self.inner.0.1.1.nb_connect(&ep) {
                Ok(()) => return Ok(()),
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }

    pub fn set_timeout(&self, timeout: Duration) -> Result<(), TimeoutError> {
        self.inner.0.1.2.set(timeout)
    }

    pub fn get_option<T>(&self) -> Result<T, OsError>
    where
        T: GetSockOpt<P>,
    {
        self.inner.0.1.1.getsockopt(self.protocol())
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint, OsError> {
        self.inner.0.1.1.getsockname()
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.inner.0.1.1.nb_recv(buf)
    }

    pub fn nb_receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint), OsError> {
        self.inner.0.1.1.nb_recvfrom(buf)
    }

    pub fn nb_receive_msg(&self, mbuf: &mut MsgBuf) -> Result<usize, OsError> {
        self.inner.0.1.1.nb_recvmsg(mbuf, self.as_ctx())
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.inner.0.1.1.nb_send(buf)
    }

    pub fn nb_send_to(&self, buf: &[u8], ep: &P::Endpoint) -> Result<usize, OsError> {
        self.inner.0.1.1.nb_sendto(buf, &EndpointRef::new(ep))
    }

    pub fn nb_send_msg(&self, mbuf: &mut MsgBuf) -> Result<usize, OsError> {
        self.inner.0.1.1.nb_sendmsg(mbuf)
    }

    pub fn protocol(&self) -> P {
        self.inner.0.1.3
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        let (_, soc, ato, _) = &self.inner.0.1;
        soc.recv(self.as_ctx(), buf, ato.get())
    }

    pub fn receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint), OsError> {
        let (_, soc, ato, _) = &self.inner.0.1;
        soc.recvfrom(self.as_ctx(), buf, ato.get())
    }

    pub fn receive_msg(&self, mbuf: &mut MsgBuf) -> Result<usize, OsError> {
        let (_, soc, ato, _) = &self.inner.0.1;
        soc.recvmsg(self.as_ctx(), mbuf, ato.get())
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint, OsError> {
        self.inner.0.1.1.getpeername()
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize, OsError> {
        let (_, soc, ato, _) = &self.inner.0.1;
        soc.send(self.as_ctx(), buf, ato.get())
    }

    pub fn send_msg(&self, mbuf: &mut MsgBuf) -> Result<usize, OsError> {
        let (_, soc, ato, _) = &self.inner.0.1;
        soc.sendmsg(self.as_ctx(), mbuf, ato.get())
    }

    pub fn send_to(&self, buf: &[u8], ep: &P::Endpoint) -> Result<usize, OsError> {
        let (_, soc, ato, _) = &self.inner.0.1;
        soc.sendto(self.as_ctx(), buf, &EndpointRef::new(ep), ato.get())
    }

    pub fn set_option<T>(&self, opt: &T) -> Result<(), OsError>
    where
        T: SetSockOpt<P>,
    {
        self.inner.0.1.1.setsockopt(self.protocol(), opt)
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<(), OsError> {
        self.inner.0.1.1.shutdown(how)
    }

    pub async fn async_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.inner.async_recv(buf).await
    }

    pub async fn async_receive_from(
        &self,
        buf: &mut [u8],
    ) -> Result<(usize, P::Endpoint), OsError> {
        self.inner.async_recvfrom(buf).await
    }

    pub async fn async_receive_msg(&self, mbuf: &mut MsgBuf) -> Result<usize, OsError> {
        self.inner.async_recvmsg(mbuf).await
    }

    pub async fn async_send(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.inner.async_send(buf).await
    }

    pub async fn async_send_msg(&self, mbuf: &mut MsgBuf) -> Result<usize, OsError> {
        self.inner.async_sendmsg(mbuf).await
    }

    pub async fn async_send_to(&self, buf: &mut [u8], ep: &P::Endpoint) -> Result<usize, OsError> {
        self.inner.async_sendto(buf, &EndpointRef::new(ep)).await
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
            inner: AsyncSocket::new(soc.ctx, soc.soc, soc.ato, soc.pro),
        }
    }
}

pub struct DgramSocketBuilder<P>
where
    P: Protocol,
{
    ctx: IoContext,
    sock_opts: LinkedList<Box<dyn SetSockOpt<P> + 'static>>,
    pro: P::Type,
}

impl<P> DgramSocketBuilder<P>
where
    P: Protocol,
{
    pub(crate) const fn new_impl(ctx: IoContext, pro: P::Type) -> Self {
        DgramSocketBuilder {
            ctx: ctx,
            sock_opts: LinkedList::new(),
            pro: pro,
        }
    }

    pub(crate) fn unbound_impl(self, pro: P) -> Result<DgramSocket<P>, OsError> {
        let ato = self.ctx.timeout();
        let soc = Socket::new(pro)?;
        Ok(DgramSocket {
            ctx: self.ctx,
            soc: soc,
            ato: ato,
            pro: pro,
        })
    }

    pub fn bind<'a, E>(self, it: E) -> Result<DgramSocket<P>, OsError>
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
