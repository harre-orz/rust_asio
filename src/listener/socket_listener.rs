use crate::error::OsError;
use crate::ffi::{self, ConnectedSocket, IntoSocket};
use crate::ops;
use crate::socket_base::Protocol;
use crate::IoContext;
use std::marker::PhantomData;
use std::time::Duration;

pub struct SocketListenerBuilder<P: Protocol, S> {
    ctx: IoContext,
    pro: P,
    ep: Option<P::Endpoint>,
    max_conns: i32,
    reuse_addr: bool,
    _marker: PhantomData<S>,
}

impl<P, S> SocketListenerBuilder<P, S>
where
    P: Protocol,
{
    pub fn bind(mut self, ep: P::Endpoint) -> Self {
        self.ep = Some(ep);
        self
    }

    pub fn listen(self) -> Result<SocketListener<P, S>, OsError> {
        let soc = ffi::socket(self.pro)?;
        if let Some(ep) = self.ep {
            ffi::bind(&soc, &ep)?;
        }
        if self.reuse_addr {
            ffi::setsockopt(&soc, libc::SOL_SOCKET, libc::SO_REUSEADDR, 1i32)?;
        }
        ffi::listen(&soc, self.max_conns)?;
        Ok(SocketListener::new_priv(self.ctx, self.pro, soc))
    }

    pub fn async_listen(self) -> Result<AsyncSocketListener<P, S>, OsError> {
        let soc = self.listen()?;
        soc.ctx.register_socket(&soc.soc);
        Ok(AsyncSocketListener {
            inner: Box::new(soc),
        })
    }

    pub fn max_conns(mut self, max_conns: i32) -> Self {
        self.max_conns = max_conns;
        self
    }

    pub fn reuse_addr(mut self, on: bool) -> Self {
        self.reuse_addr = on;
        self
    }
}

pub struct SocketListener<P, S> {
    ctx: IoContext,
    pro: P,
    soc: ConnectedSocket,
    wait_timeout: Duration,
    _marker: PhantomData<S>,
}

impl<P, S> SocketListener<P, S>
where
    P: Protocol,
{
    pub fn new(ctx: &IoContext, pro: P) -> SocketListenerBuilder<P, S> {
        SocketListenerBuilder {
            ctx: ctx.clone(),
            pro: pro,
            ep: None,
            max_conns: libc::SOMAXCONN,
            reuse_addr: false,
            _marker: PhantomData,
        }
    }

    fn new_priv(ctx: IoContext, pro: P, soc: ConnectedSocket) -> Self {
        Self {
            ctx: ctx,
            pro: pro,
            soc: soc,
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
    inner: Box<SocketListener<P, S>>,
}

impl<P, S> Drop for AsyncSocketListener<P, S> {
    fn drop(&mut self) {
        self.inner.ctx.deregister_socket(&self.inner.soc);
    }
}

impl<P, S> AsyncSocketListener<P, S>
where
    P: Protocol,
{
    pub fn as_ctx(&self) -> &IoContext {
        self.inner.as_ctx()
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint, OsError> {
        self.inner.local_endpoint()
    }

    pub fn protocol(&self) -> P {
        self.inner.protocol()
    }
}

impl<P, S> AsyncSocketListener<P, S>
where
    P: Protocol,
    Self: IntoSocket<Socket = S>,
{
    pub fn nb_accept(&self) -> Result<(S, P::Endpoint), OsError> {
        let (soc, ep) = ffi::accept(&self.inner.soc)?;
        Ok((self.into_socket(soc), ep))
    }

    pub fn accept(&self) -> Result<(S, P::Endpoint), OsError> {
        let (soc, ep) = ops::accept(&self.inner.ctx, &self.inner.soc, self.inner.wait_timeout)?;
        Ok((self.into_socket(soc), ep))
    }

    pub async fn async_accept(&self) -> Result<(S, P::Endpoint), OsError> {
        let (soc, ep) =
            ops::async_accept(&self.inner.ctx, &self.inner.soc, self.inner.wait_timeout).await?;
        Ok((self.into_socket(soc), ep))
    }
}
