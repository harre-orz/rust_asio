use crate::IoContext;
use crate::buffer::{AsyncIoStream, IoStream};
use crate::error::{OsError, Result};
use crate::core;
use crate::core::AsyncSocket;
use crate::core::{Socket, Timeout};
use crate::socket_base::{Endpoints, GetSockOpt, Protocol, SetSockOpt, Shutdown};
use std::any::Any;
use std::collections::LinkedList;
use std::time::Duration;

pub struct AsyncStreamSocket<P>
where
    P: Protocol,
{
    soc: AsyncSocket,
    pro: P,
    timeout: Timeout,
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
        self.soc.as_socket().receive(buf)
    }

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.as_socket().read(buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.as_socket().send(buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize> {
        self.soc.as_socket().write(buf)
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
        self.timeout = Timeout::from_duration(timeout)
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        self.soc.as_socket().shutdown(how)
    }

    pub async fn async_read_some(&self, buf: &mut [u8]) -> Result<usize> {
        core::async_read_some(&self.soc, buf, self.timeout).await
    }

    pub async fn async_receive(&self, buf: &mut [u8]) -> Result<usize> {
        core::async_receive(&self.soc, buf, self.timeout).await
    }

    pub async fn async_send(&self, buf: &[u8]) -> Result<usize> {
        core::async_send(&self.soc, buf, self.timeout).await
    }

    pub async fn async_write_some(&self, buf: &[u8]) -> Result<usize> {
        core::async_write_some(&self.soc, buf, self.timeout).await
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

pub struct StreamSocket<P>
where
    P: Protocol,
{
    ctx: IoContext,
    soc: Socket,
    pro: P,
    timeout: Timeout,
}

impl<P> StreamSocket<P>
where
    P: Protocol,
{
    pub(crate) fn new_impl(ctx: IoContext, soc: Socket, pro: P) -> Self {
        Self {
            soc: soc,
            ctx: ctx,
            pro: pro,
            timeout: Timeout::infinite(),
        }
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
        self.soc.receive(buf)
    }

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize> {
        self.soc.read(buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        self.soc.send(buf)
    }

    pub fn nb_write_some(&self, buf: &[u8]) -> Result<usize> {
        self.soc.write(buf)
    }

    pub const fn protocol(&self) -> P {
        self.pro
    }

    pub fn read_some(&self, buf: &mut [u8]) -> Result<usize> {
        core::read_some(&self.ctx, &self.soc, buf, self.timeout)
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize> {
        core::receive(&self.ctx, &self.soc, buf, self.timeout)
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint> {
        self.soc.getpeername()
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize> {
        core::send(&self.ctx, &self.soc, buf, self.timeout)
    }

    pub fn set_option<T>(&self, opt: &T) -> Result<()>
    where
        T: SetSockOpt<P>,
    {
        self.soc.setsockopt(self.pro, opt)
    }

    pub const fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = Timeout::from_duration(timeout)
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        self.soc.shutdown(how)
    }

    pub fn write_some(&self, buf: &[u8]) -> Result<usize> {
        core::write_some(&self.ctx, &self.soc, buf, self.timeout)
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
            timeout: soc.timeout,
        }
    }
}

pub struct StreamSocketBuilder<P>
where
    P: Protocol,
{
    pro: P::Type,
    ctx: IoContext,
    timeout: Timeout,
    sock_opts: LinkedList<Box<dyn SetSockOpt<P>>>,
}

unsafe impl<P> Send for StreamSocketBuilder<P> where P: Protocol + Send {}
unsafe impl<P> Sync for StreamSocketBuilder<P> where P: Protocol + Sync {}

impl<P: Protocol> StreamSocketBuilder<P> {
    pub(crate) fn new_impl(ctx: IoContext, pro: P::Type) -> Self {
        Self {
            pro: pro,
            ctx: ctx,
            timeout: Timeout::infinite(),
            sock_opts: LinkedList::new(),
        }
    }

    pub fn nb_connect<'a, E>(self, eps: E) -> Result<StreamSocket<P>>
    where
        P: 'a,
        E: Endpoints<'a, P>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in eps.endpoints() {
            let pro = P::new(&ep, self.pro);
            let soc = Socket::new(pro)?;
            match soc.connect(&ep) {
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
        P: 'a,
        E: Endpoints<'a, P>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in eps.endpoints() {
            let pro = P::new(&ep, self.pro);
            let soc = Socket::new(pro)?;
            for opt in &self.sock_opts {
                soc.setsockopt(pro, opt.as_ref())?;
            }
            match core::connect(&self.ctx, &soc, &ep, self.timeout) {
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
        P: 'a,
        E: Endpoints<'a, P>,
    {
        let mut last_err = OsError::OPERATION_CANCELED;
        for ep in eps.endpoints() {
            let pro = P::new(&ep, self.pro);
            let soc = Socket::new(pro)?;
            for opt in &self.sock_opts {
                soc.setsockopt(pro, opt.as_ref())?;
            }
            let soc = AsyncSocket::new(self.ctx.clone(), soc);
            match core::async_connect(&soc, &ep, self.timeout).await {
                Ok(_) => {
                    return Ok(AsyncStreamSocket {
                        soc: soc,
                        pro: pro,
                        timeout: self.timeout,
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
            timeout: self.timeout,
            sock_opts: sock_opts,
        }
    }
}
