use crate::error::OsError;
use crate::executor::{AsyncSocket, IoContext};
use crate::ffi::{self, ConnectedSocket, IntoSocket, Timeout};
use crate::ops;
use crate::socket_base::Protocol;
use crate::stream::{AsyncStreamSocket, StreamSocket};
use crate::dgram::{AsyncSeqPacketSocket, SeqPacketSocket};
use std::marker::PhantomData;

pub struct SocketListenerBuilder<'a, P: Protocol> {
    ctx: &'a IoContext,
    soc: ConnectedSocket,
    pro: P,
    max_conns: i32,
}

impl<'a, P> SocketListenerBuilder<'a, P>
where
    P: Protocol,
{
    pub fn new(ctx: &'a IoContext, pro: P) -> Result<Self, OsError> {
        let soc = ffi::socket(pro)?;
        Ok(Self {
            ctx,
            soc,
            pro,
            max_conns: libc::SOMAXCONN,
        })
    }

    pub fn max_conns(mut self, max_conns: i32) -> Self {
        self.max_conns = max_conns;
        self
    }

    pub fn bind<E>(self, ep: E) -> Result<Self, OsError>
    where
        E: AsRef<P::Endpoint>,
    {
        ffi::bind(&self.soc, ep.as_ref())?;
        Ok(self)
    }

    pub fn reuse_addr(self, on: bool) -> Result<Self, OsError> {
        let on = if on { 1i32 } else { 0i32 };
        ffi::setsockopt(&self.soc, libc::SOL_SOCKET, libc::SO_REUSEADDR, on)?;
        Ok(self)
    }

    pub fn listen<S>(self) -> Result<SocketListener<P, S>, OsError>
    {
        ffi::listen(&self.soc, self.max_conns)?;
        Ok(SocketListener {
            ctx: self.ctx.clone(),
            soc: self.soc,
            pro: self.pro,
            read_timeout: Timeout::new(),
            _marker: PhantomData,
        })
    }

    pub fn listen_async<S>(self) -> Result<AsyncSocketListener<P, S>, OsError>
    {
        ffi::listen(&self.soc, self.max_conns)?;
        Ok(AsyncSocketListener {
            soc: AsyncSocket::new(self.ctx.clone(), self.soc),
            pro: self.pro,
            read_timeout: Timeout::new(),
            _marker: PhantomData,
        })
    }
}

pub struct SocketListener<P, S> {
    ctx: IoContext,
    soc: ConnectedSocket,
    pro: P,
    read_timeout: Timeout,
    _marker: PhantomData<S>,
}

impl<P, S> SocketListener<P, S>
where
    P: Protocol,
{
    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn close(self) -> Result<(), OsError> {
        self.soc.close()
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getsockname(&self.soc)
    }

    pub fn protocol(&self) -> P {
        self.pro
    }
}

impl<P, S> SocketListener<P, S>
where
    P: Protocol,
    Self: IntoSocket<Socket = S>,
{
    pub fn nb_accept(&self) -> Result<(S, P::Endpoint), OsError> {
        let (soc, ep) = ffi::accept(&self.soc)?;
        Ok((self.into_socket(soc), ep))
    }

    pub fn accept(&self) -> Result<(S, P::Endpoint), OsError> {
        let (soc, ep) = ops::accept(&self.soc, self.read_timeout, &self.ctx)?;
        Ok((self.into_socket(soc), ep))
    }
}

pub struct AsyncSocketListener<P, S> {
    soc: AsyncSocket,
    pro: P,
    read_timeout: Timeout,
    _marker: PhantomData<S>,
}

impl<P, S> AsyncSocketListener<P, S>
where
    P: Protocol,
{
    pub fn as_ctx(&self) -> &IoContext {
        self.soc.as_ctx()
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getsockname(self.soc.as_socket())
    }

    pub fn protocol(&self) -> P {
        self.pro
    }
}

impl<P> From<SocketListener<P, StreamSocket<P>>> for AsyncSocketListener<P, AsyncStreamSocket<P>>
{
    fn from(soc: SocketListener<P, StreamSocket<P>>) -> Self {
        let SocketListener {
            ctx,
            soc,
            pro,
            read_timeout,
            _marker: _,
        } = soc;
        Self {
            soc: AsyncSocket::new(ctx, soc),
            pro,
            read_timeout,
            _marker: PhantomData,
        }
    }
}

impl<P> From<SocketListener<P, SeqPacketSocket<P>>> for AsyncSocketListener<P, AsyncSeqPacketSocket<P>>
{
    fn from(soc: SocketListener<P, SeqPacketSocket<P>>) -> Self {
        let SocketListener {
            ctx,
            soc,
            pro,
            read_timeout,
            _marker: _,
        } = soc;
        Self {
            soc: AsyncSocket::new(ctx, soc),
            pro,
            read_timeout,
            _marker: PhantomData,
        }
    }
}

impl<P, S> AsyncSocketListener<P, S>
where
    P: Protocol,
    Self: IntoSocket<Socket = S>,
{
    pub fn nb_accept(&self) -> Result<(S, P::Endpoint), OsError> {
        let (soc, ep) = ffi::accept(self.soc.as_socket())?;
        Ok((self.into_socket(soc), ep))
    }

    pub fn accept(&self) -> Result<(S, P::Endpoint), OsError> {
        let (soc, ep) = ops::accept(self.soc.as_socket(), self.read_timeout, &self.soc.as_ctx())?;
        Ok((self.into_socket(soc), ep))
    }

    pub async fn async_accept(&self) -> Result<(S, P::Endpoint), OsError> {
        let (soc, ep) = ops::async_accept(&self.soc, self.read_timeout).await?;
        Ok((self.into_socket(soc), ep))
    }
}
