use crate::core::IoContext;
use crate::error::OsError;
use crate::primitive::{AtomicTimeout, Socket, Timeout, TimeoutError};
use crate::socket::AsyncSocket;
use crate::socket_base::{EndpointRef, GetSockOpt, Protocol, SetSockOpt, Shutdown};
use std::time::Duration;

pub struct SeqPacketSocket<P>
where
    P: Protocol,
{
    ctx: IoContext,
    soc: Socket,
    ato: AtomicTimeout,
    pro: P,
}

impl<P> SeqPacketSocket<P>
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

    pub fn get_option<T>(&self) -> Result<T, OsError>
    where
        T: GetSockOpt<P>,
    {
        self.soc.getsockopt(self.pro)
    }

    pub fn set_timeout(&self, timer: Duration) -> Result<(), TimeoutError> {
        self.ato.set(timer)
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint, OsError> {
        self.soc.getsockname()
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.soc.nb_recv(buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.soc.nb_send(buf)
    }

    pub fn close(self) -> Result<(), OsError> {
        self.soc.close()
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<(), OsError> {
        self.soc.shutdown(how)
    }

    pub const fn protocol(&self) -> P {
        self.pro
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.soc.recv(&self.ctx, buf, self.ato.get())
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
        self.soc.send(&self.ctx, buf, self.ato.get())
    }
}

pub struct AsyncSeqPacketSocket<P>
where
    P: Protocol,
{
    inner: AsyncSocket<(AtomicTimeout, P)>,
}

impl<P> AsyncSeqPacketSocket<P>
where
    P: Protocol,
{
    pub fn as_ctx(&self) -> &IoContext {
        self.inner.as_ctx()
    }

    pub fn set_timeout(&mut self, timer: Duration) -> Result<(), TimeoutError> {
        self.inner.as_data().0.set(timer)
    }

    fn timeout(&self) -> Timeout {
        self.inner.as_data().0.get()
    }

    pub fn get_option<T>(&self) -> Result<T, OsError>
    where
        T: GetSockOpt<P>,
    {
        self.inner.as_socket().getsockopt(self.protocol())
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint, OsError> {
        self.inner.as_socket().getsockname()
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.inner.as_socket().nb_recv(buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.inner.as_socket().nb_send(buf)
    }

    pub fn protocol(&self) -> P {
        self.inner.as_data().1
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.inner
            .as_socket()
            .recv(self.as_ctx(), buf, self.timeout())
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint, OsError> {
        self.inner.as_socket().getpeername()
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.inner
            .as_socket()
            .send(self.as_ctx(), buf, self.timeout())
    }

    pub fn set_option<T>(&self, opt: &T) -> Result<(), OsError>
    where
        T: SetSockOpt<P>,
    {
        self.inner.as_socket().setsockopt(self.protocol(), opt)
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<(), OsError> {
        self.inner.as_socket().shutdown(how)
    }

    pub async fn async_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.inner.async_recv(buf, self.timeout()).await
    }

    pub async fn async_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.inner.async_send(buf, self.timeout()).await
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
            inner: AsyncSocket::new(soc.ctx, soc.soc, (soc.ato, soc.pro)),
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

    pub fn connect<'a, E>(self, eps: E) -> Result<SeqPacketSocket<P>, OsError>
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
