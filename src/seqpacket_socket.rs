use crate::IoContext;
use crate::error::{OsError, Result};
use crate::exec::AsyncSocket;
use crate::ops::{self};
use crate::sockaddr::SockAddr;
use crate::socket::{Shutdown, Socket, Timeout};
use crate::socket_base::{Endpoints, Protocol};
use std::marker::PhantomData;
use std::time::Duration;

pub struct AsyncSeqPacketSocket<P>
where
    P: Protocol,
{
    soc: AsyncSocket,
    timeout: Timeout,
    _marker: PhantomData<P>,
}

impl<P> AsyncSeqPacketSocket<P>
where
    P: Protocol,
{
    pub const fn as_ctx(&self) -> &IoContext {
        self.soc.as_ctx()
    }

    pub const fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = Timeout::from_duration(timeout)
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
        ops::async_receive(&self.soc, buf, self.timeout).await
    }

    pub async fn async_send(&self, buf: &[u8]) -> Result<usize> {
        ops::async_send(&self.soc, buf, self.timeout).await
    }
}

pub struct SeqPacketSocket<P>
where
    P: Protocol,
{
    soc: Socket,
    ctx: IoContext,
    timeout: Timeout,
    _marker: PhantomData<P>,
}

impl<P> SeqPacketSocket<P>
where
    P: Protocol,
{
    pub(crate) fn new_impl(ctx: IoContext, soc: Socket) -> Self {
        Self {
            soc: soc,
            ctx: ctx,
            timeout: Timeout::infinite(),
            _marker: PhantomData,
        }
    }

    pub const fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub const fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = Timeout::from_duration(timeout)
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
        ops::receive(&self.ctx, &self.soc, buf, self.timeout)
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.getpeername()
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize> {
        ops::send(&self.ctx, &self.soc, buf, self.timeout)
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
            soc: AsyncSocket::new(soc.ctx, soc.soc),
            timeout: soc.timeout,
            _marker: PhantomData,
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

    pub fn protocol_type(&self) -> P::Type {
        self.pro
    }
}
