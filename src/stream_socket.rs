use crate::buffer::{AsyncIoStream, IoStream};
use crate::core::IoContext;
use crate::error::OsError;
use crate::primitive::{AtomicTimeout, DurationOverflowError, Socket};
use crate::socket::AsyncSocket;
use crate::socket_base::{EndpointRef, GetSockOpt, Protocol, SetSockOpt, Shutdown};
use std::any::Any;
use std::collections::LinkedList;
use std::time::Duration;

pub struct StreamSocket<P>
where
    P: Protocol,
{
    ctx: IoContext,
    soc: Socket,
    ato: AtomicTimeout,
    pro: P,
}

impl<P> StreamSocket<P>
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

    pub fn close(self) -> Result<(), OsError> {
        self.soc.close()
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

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.soc.nb_read(buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.soc.nb_send(buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.soc.nb_write(buf)
    }

    pub const fn protocol(&self) -> P {
        self.pro
    }

    pub fn read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.soc.read(&self.ctx, buf, self.ato.get())
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.soc.recv(&self.ctx, buf, self.ato.get())
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint, OsError> {
        self.soc.getpeername()
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.soc.send(&self.ctx, buf, self.ato.get())
    }

    pub fn set_option<T>(&self, opt: &T) -> Result<(), OsError>
    where
        T: SetSockOpt<P>,
    {
        self.soc.setsockopt(self.pro, opt)
    }

    pub fn set_timeout(&mut self, timer: Duration) -> Result<(), DurationOverflowError> {
        self.ato.set(timer)
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<(), OsError> {
        self.soc.shutdown(how)
    }

    pub fn write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.soc.write(&self.ctx, buf, self.ato.get())
    }
}

impl<P> IoStream for StreamSocket<P>
where
    P: Protocol,
{
    type Error = OsError;

    fn read(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.read_some(buf)
    }

    fn write(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.write_some(buf)
    }
}
pub struct AsyncStreamSocket<P>
where
    P: Protocol,
{
    inner: AsyncSocket<P>,
}

impl<P> AsyncStreamSocket<P>
where
    P: Protocol,
{
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

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.inner.0.1.1.nb_read(buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.inner.0.1.1.nb_send(buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.inner.0.1.1.nb_write(buf)
    }

    pub fn protocol(&self) -> P {
        self.inner.0.1.3
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint, OsError> {
        self.inner.0.1.1.getpeername()
    }

    pub fn set_option<T>(&self, opt: &T) -> Result<(), OsError>
    where
        T: SetSockOpt<P>,
    {
        self.inner.0.1.1.setsockopt(self.protocol(), opt)
    }

    pub fn set_timeout(&mut self, timeout: Duration) -> Result<(), DurationOverflowError> {
        self.inner.0.1.2.set(timeout)
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<(), OsError> {
        self.inner.0.1.1.shutdown(how)
    }

    pub async fn async_read_some(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.inner.async_read(buf).await
    }

    pub async fn async_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.inner.async_recv(buf).await
    }

    pub async fn async_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.inner.async_send(buf).await
    }

    pub async fn async_write_some(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.inner.async_write(buf).await
    }
}

impl<P> AsyncIoStream for AsyncStreamSocket<P>
where
    P: Protocol,
{
    type Error = OsError;

    async fn async_read(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        self.async_read_some(buf).await
    }

    async fn async_write(&self, buf: &[u8]) -> Result<usize, OsError> {
        self.async_write_some(buf).await
    }
}

/// Converts Asynchronous socket.
///
/// # Examples
///
/// ```no_run
/// use asyncio::IoContext;
/// use asyncio::local::{LocalStreamEndpoint, LocalStreamSocket, AsyncLocalStreamSocket};
/// use std::path::Path;
///
/// let ctx = &IoContext::new().unwrap();
/// let ep = LocalStreamEndpoint::new(Path::new("/foo/bar")).unwrap();
/// let soc = LocalStreamSocket::new(ctx).connect(&ep).unwrap();
/// let soc = AsyncLocalStreamSocket::from(soc);
/// ```
impl<P> From<StreamSocket<P>> for AsyncStreamSocket<P>
where
    P: Protocol,
{
    fn from(soc: StreamSocket<P>) -> Self {
        Self {
            inner: AsyncSocket::new(soc.ctx, soc.soc, soc.ato, soc.pro),
        }
    }
}

pub struct StreamSocketBuilder<P>
where
    P: Protocol,
{
    pro: P::Type,
    ctx: IoContext,
    ato: AtomicTimeout,
    sock_opts: LinkedList<Box<dyn SetSockOpt<P>>>,
}

unsafe impl<P> Send for StreamSocketBuilder<P> where P: Protocol + Send {}
unsafe impl<P> Sync for StreamSocketBuilder<P> where P: Protocol + Sync {}

impl<P: Protocol> StreamSocketBuilder<P> {
    pub(crate) fn new_impl(ctx: IoContext, pro: P::Type) -> Self {
        let ato = ctx.timeout();
        Self {
            pro: pro,
            ctx: ctx,
            ato: ato,
            sock_opts: LinkedList::new(),
        }
    }

    pub fn nb_connect<'a, E>(self, eps: E) -> Result<StreamSocket<P>, OsError>
    where
        E: IntoIterator<Item = EndpointRef<'a, <P as Protocol>::Endpoint>>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in eps {
            let pro = P::new(&ep, self.pro);
            let soc = Socket::new(pro)?;
            match soc.nb_connect(&ep) {
                Ok(_) => {
                    return Ok(StreamSocket::new_impl(self.ctx, soc, pro));
                }
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }

    pub fn connect<'a, E>(self, eps: E) -> Result<StreamSocket<P>, OsError>
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
            match soc.connect(&self.ctx, &ep, self.ato.get()) {
                Ok(_) => {
                    return Ok(StreamSocket::new_impl(self.ctx, soc, pro));
                }
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
    }

    pub fn protocol_type(&self) -> P::Type {
        self.pro
    }

    pub async fn async_connect<'a, E>(self, eps: E) -> Result<AsyncStreamSocket<P>, OsError>
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
            let inner = AsyncSocket::new(self.ctx.clone(), soc, self.ato.clone(), pro);
            match inner.async_connect(&ep).await {
                Ok(_) => {
                    return Ok(AsyncStreamSocket { inner: inner });
                }
                Err(err) => last_err = err,
            }
        }
        Err(last_err)
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
            ato: self.ato,
            sock_opts: sock_opts,
        }
    }
}
