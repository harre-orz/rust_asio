use crate::error::OsError;
use crate::executor::{AsyncSocket, IoContext};
use crate::ffi::{self, ConnectedSocket, IntoSocket};
use crate::ops;
use crate::socket_base::Protocol;
use std::marker::PhantomData;
use std::time::Duration;

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
            ctx: ctx,
            soc: soc,
            pro: pro,
            max_conns: libc::SOMAXCONN,
        })
    }

    pub fn bind(self, ep: &P::Endpoint) -> Result<Self, OsError> {
        ffi::bind(&self.soc, ep)?;
        Ok(self)
    }

    pub fn max_conns(mut self, max_conns: i32) -> Self {
        self.max_conns = max_conns;
        self
    }

    pub fn reuse_addr(self, on: bool) -> Result<Self, OsError> {
        let on = if on { 1i32 } else { 0i32 };
        ffi::setsockopt(&self.soc, libc::SOL_SOCKET, libc::SO_REUSEADDR, on)?;
        Ok(self)
    }

    pub fn listen<S>(self) -> Result<SocketListener<P, S>, OsError> {
        ffi::listen(&self.soc, self.max_conns)?;
        Ok(SocketListener::new_priv(self.ctx, self.soc, self.pro))
    }

    pub fn async_listen<S>(self) -> Result<AsyncSocketListener<P, S>, OsError> {
        ffi::listen(&self.soc, self.max_conns)?;
        Ok(AsyncSocketListener::new_priv(self.ctx, self.soc, self.pro))
    }
}

pub struct SocketListener<P, S> {
    ctx: IoContext,
    soc: ConnectedSocket,
    pro: P,
    wait_timeout: Duration,
    _marker: PhantomData<S>,
}

impl<P, S> SocketListener<P, S>
where
    P: Protocol,
{
    fn new_priv(ctx: &IoContext, soc: ConnectedSocket, pro: P) -> Self {
        Self {
            ctx: ctx.clone(),
            soc: soc,
            pro: pro,
            wait_timeout: Duration::MAX,
            _marker: PhantomData,
        }
    }

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
        let (soc, ep) = ops::accept(&self.ctx, &self.soc, self.wait_timeout)?;
        Ok((self.into_socket(soc), ep))
    }
}


pub struct AsyncSocketListener<P, S> {
    soc: AsyncSocket,
    pro: P,
    wait_timeout: Duration,
    _marker: PhantomData<S>,
}

impl<P, S> AsyncSocketListener<P, S>
where
    P: Protocol,
{
    fn new_priv(ctx: &IoContext, soc: ConnectedSocket, pro: P) -> Self {
        let soc = ctx.async_socket(soc);
        Self {
            soc: soc,
            pro: pro,
            wait_timeout: Duration::MAX,
            _marker: PhantomData,
        }
    }

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
        let (soc, ep) = ops::accept(&self.soc.as_ctx(), self.soc.as_socket(), self.wait_timeout)?;
        Ok((self.into_socket(soc), ep))
    }

    pub async fn async_accept(&self) -> Result<(S, P::Endpoint), OsError> {
        let (soc, ep) = ops::async_accept(&self.soc, self.wait_timeout).await?;
        Ok((self.into_socket(soc), ep))
    }
}
