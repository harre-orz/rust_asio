use crate::buffer::{AsyncIoStream, IoStream};
use crate::core::IoContext;
use crate::error::{OsError, Result};
use crate::primitive::{Timeout, Socket};
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
    pro: P,
    t: Timeout,
}

impl<P> StreamSocket<P>
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

    pub fn close(self) -> Result<()> {
        self.soc.close()
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

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.nb_read(buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.nb_send(buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize> {
        self.soc.nb_write(buf)
    }

    pub const fn protocol(&self) -> P {
        self.pro
    }

    pub fn read_some(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.read(&self.ctx, buf, self.t)
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.recv(&self.ctx, buf, self.t)
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.getpeername()
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.send(&self.ctx, buf, self.t)
    }

    pub fn set_option<T>(&self, opt: &T) -> Result<()>
    where
        T: SetSockOpt<P>,
    {
        self.soc.setsockopt(self.pro, opt)
    }

    pub const fn set_timeout(&mut self, timeout: Duration) {
        self.t = Timeout::from_duration(timeout)
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        self.soc.shutdown(how)
    }

    pub fn write_some(&self, buf: &[u8]) -> Result<usize> {
        self.soc.write(&self.ctx, buf, self.t)
    }
}

impl<P> IoStream for StreamSocket<P>
where
    P: Protocol,
{
    type Error = OsError;

    fn read(&self, buf: &mut [u8]) -> Result<usize> {
        self.read_some(buf)
    }

    fn write(&self, buf: &[u8]) -> Result<usize> {
        self.write_some(buf)
    }
}
pub struct AsyncStreamSocket<P>
where
    P: Protocol,
{
    soc: AsyncSocket,
    pro: P,
    t: Timeout,
}

impl<P> AsyncStreamSocket<P>
where
    P: Protocol,
{
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

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.as_socket().nb_read(buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.as_socket().nb_send(buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize> {
        self.soc.as_socket().nb_write(buf)
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

    pub const fn set_timeout(&mut self, timeout: Duration) {
        self.t = Timeout::from_duration(timeout)
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        self.soc.as_socket().shutdown(how)
    }

    pub async fn async_read_some(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.async_read(buf, self.t).await
    }

    pub async fn async_receive(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.async_recv(buf, self.t).await
    }

    pub async fn async_send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.async_send(buf, self.t).await
    }

    pub async fn async_write_some(&self, buf: &[u8]) -> Result<usize> {
        self.soc.async_write(buf, self.t).await
    }
}

impl<P> AsyncIoStream for AsyncStreamSocket<P>
where
    P: Protocol,
{
    type Error = OsError;

    async fn async_read(&self, buf: &mut [u8]) -> Result<usize> {
        self.async_read_some(buf).await
    }

    async fn async_write(&self, buf: &[u8]) -> Result<usize> {
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
            soc: AsyncSocket::new(soc.ctx, soc.soc),
            pro: soc.pro,
            t: soc.t,
        }
    }
}

pub struct StreamSocketBuilder<P>
where
    P: Protocol,
{
    pro: P::Type,
    ctx: IoContext,
    t: Timeout,
    sock_opts: LinkedList<Box<dyn SetSockOpt<P>>>,
}

unsafe impl<P> Send for StreamSocketBuilder<P> where P: Protocol + Send {}
unsafe impl<P> Sync for StreamSocketBuilder<P> where P: Protocol + Sync {}

impl<P: Protocol> StreamSocketBuilder<P> {
    pub(crate) fn new_impl(ctx: IoContext, pro: P::Type) -> Self {
        Self {
            pro: pro,
            ctx: ctx,
            t: Timeout::INFINITE,
            sock_opts: LinkedList::new(),
        }
    }

    pub fn nb_connect<'a, E>(self, eps: E) -> Result<StreamSocket<P>>
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

    pub fn connect<'a, E>(self, eps: E) -> Result<StreamSocket<P>>
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
            match soc.connect(&self.ctx, &ep, self.t) {
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

    pub async fn async_connect<'a, E>(self, eps: E) -> Result<AsyncStreamSocket<P>>
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
            let soc = AsyncSocket::new(self.ctx.clone(), soc);
            match soc.async_connect(&ep, self.t).await {
                Ok(_) => {
                    return Ok(AsyncStreamSocket {
                        soc: soc,
                        pro: pro,
                        t: self.t,
                    });
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
            t: self.t,
            sock_opts: sock_opts,
        }
    }
}
