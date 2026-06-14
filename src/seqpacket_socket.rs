use crate::core::IoContext;
use crate::error::{OsError, Result};
use crate::primitive::{Socket, Timeout};
use crate::socket::AsyncSocket;
use crate::socket_base::{EndpointRef, GetSockOpt, Protocol, SetSockOpt, Shutdown};
use std::time::Duration;

pub struct SeqPacketSocket<P>
where
    P: Protocol,
{
    ctx: IoContext,
    soc: Socket,
    pro: P,
    t: Timeout,
}

impl<P> SeqPacketSocket<P>
where
    P: Protocol,
{
    pub(crate) fn new_impl(ctx: IoContext, soc: Socket, pro: P) -> Self {
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

    pub fn get_option<T>(&self) -> Result<T>
    where
        T: GetSockOpt<P>,
    {
        self.soc.getsockopt(self.pro)
    }

    pub const fn set_timeout(&mut self, timeout: Duration) {
        self.t = Timeout::from_duration(timeout)
    }
    pub fn local_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.getsockname()
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.nb_recv(buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.nb_send(buf)
    }

    pub fn close(self) -> Result<()> {
        self.soc.close()
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        self.soc.shutdown(how)
    }

    pub const fn protocol(&self) -> P {
        self.pro
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.recv(&self.ctx, buf, self.t)
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
        self.soc.send(&self.ctx, buf, self.t)
    }
}

pub struct AsyncSeqPacketSocket<P>
where
    P: Protocol,
{
    soc: AsyncSocket<()>,
    pro: P,
    t: Timeout,
}

impl<P> AsyncSeqPacketSocket<P>
where
    P: Protocol,
{
    pub fn as_ctx(&self) -> &IoContext {
        self.soc.as_ctx()
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

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.as_socket().nb_send(buf)
    }

    pub const fn protocol(&self) -> P {
        self.pro
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.as_socket().recv(self.as_ctx(), buf, self.t)
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.as_socket().getpeername()
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.as_socket().send(self.as_ctx(), buf, self.t)
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

    pub async fn async_send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.async_send(buf, self.t).await
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
impl<P> From<SeqPacketSocket<P>> for AsyncSeqPacketSocket<P>
where
    P: Protocol,
{
    fn from(soc: SeqPacketSocket<P>) -> Self {
        Self {
            soc: AsyncSocket::new(soc.ctx, soc.soc, ()),
            pro: soc.pro,
            t: soc.t,
        }
    }
}

pub struct SeqPacketSocketBuilder<P>
where
    P: Protocol,
{
    ctx: IoContext,
    pro: P::Type,
}

impl<P> SeqPacketSocketBuilder<P>
where
    P: Protocol,
{
    pub(crate) const fn new_impl(ctx: IoContext, pro: P::Type) -> Self {
        Self { ctx: ctx, pro: pro }
    }

    pub fn connect<'a, E>(self, eps: E) -> Result<SeqPacketSocket<P>>
    where
        E: IntoIterator<Item = EndpointRef<'a, <P as Protocol>::Endpoint>>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in eps {
            let pro = P::new(&ep, self.pro);
            let soc = Socket::new(pro)?;
            match soc.nb_connect(&ep) {
                Ok(_) => return Ok(SeqPacketSocket::new_impl(self.ctx, soc, pro)),
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }

    pub fn protocol_type(&self) -> P::Type {
        self.pro
    }
}
