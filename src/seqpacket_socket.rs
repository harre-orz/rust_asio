use crate::poll::Timeout;
use crate::error::{OsError, Result};
use crate::socket::{AsyncSocket, Socket};
use crate::socket_base::{Endpoints, GetSockOpt, Protocol, SetSockOpt, Shutdown};
use crate::{IoContext};
use std::time::Duration;

pub struct SeqPacketSocket<P>
where
    P: Protocol,
{
    soc: Socket,
    pro: P,
    timeout: Timeout,
}

impl<P> SeqPacketSocket<P>
where
    P: Protocol,
{
    pub(crate) fn new_impl(soc: Socket, pro: P) -> Self {
        Self {
            soc: soc,
            pro: pro,
            timeout: Timeout::infinite(),
        }
    }

    pub const fn as_ctx(&self) -> &IoContext {
        self.soc.as_ctx()
    }

    pub fn get_option<T>(&self) -> Result<T>
    where
        T: GetSockOpt<P>,
    {
        self.soc.getsockopt(self.pro)
    }

    pub const fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = Timeout::from_duration(timeout)
    }
    pub fn local_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.getsockname()
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.nb_receive(buf)
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
        self.soc.receive(buf, self.timeout)
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
        self.soc.send(buf, self.timeout)
    }
}


pub struct AsyncSeqPacketSocket<P>
where
    P: Protocol,
{
    soc: AsyncSocket,
    pro: P,
    timeout: Timeout,
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

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.as_socket().nb_send(buf)
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

    pub async fn async_send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.async_send(buf, self.timeout).await
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
            soc: AsyncSocket::new(soc.soc),
            pro: soc.pro,
            timeout: soc.timeout,
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
            let pro = P::new(&ep, self.pro);
            let soc = Socket::new(&self.ctx, pro)?;
            match soc.nb_connect(&ep) {
                Ok(_) => return Ok(SeqPacketSocket::new_impl(soc, pro)),
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }

    pub fn protocol_type(&self) -> P::Type {
        self.pro
    }
}
