use super::{ConnectedSocket, IntoConnectedSocket};
use crate::{ffi, OsError, IoContext, Protocol, YieldContext};
use std::marker::PhantomData;
use std::os::fd::OwnedFd;
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

    pub fn max_conns(mut self, max_conns: i32) -> Self {
        self.max_conns = max_conns;
        self
    }

    pub fn reuse_addr(mut self, on: bool) -> Self {
        self.reuse_addr = on;
        self
    }

    pub fn listen(self) -> Result<SocketListener<P, S>, OsError> {
        let soc = ffi::socket(self.pro)?;
        if let Some(ep) = self.ep {
            ffi::bind(&soc, &ep)?;
        }
        if self.reuse_addr {
            ffi::setsockopt(&soc,
                libc::SOL_SOCKET,
                libc::SO_REUSEADDR,
                1i32,
            )?;
        }
        ffi::listen(&soc, self.max_conns)?;
        Ok(SocketListener::new_priv(self.ctx, self.pro, soc))
    }
}

pub struct SocketListener<P, S> {
    ctx: IoContext,
    pro: P,
    soc: OwnedFd,
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

    fn new_priv(ctx: IoContext, pro: P, soc: OwnedFd) -> Self {
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

    pub fn protocol(&self) -> P {
        self.pro
    }
}

impl<P, S> SocketListener<P, S>
where
    P: Protocol,
    Self: IntoConnectedSocket<Socket = S>,
{
    pub fn nb_accept(&self) -> Result<(S, P::Endpoint), OsError> {
        let (soc, ep) = ffi::accept(&self.soc)?;
        let conn = ConnectedSocket {
            ctx: self.as_ctx().clone(),
            soc: soc,
        };
        Ok((self.into_connected_socket(conn), ep))
    }

    pub fn accept(&self, yield_ctx: &mut YieldContext) -> Result<(S, P::Endpoint), OsError> {
        loop {
            match ffi::accept(&self.soc) {
                Ok((soc, ep)) => {
                    let conn = ConnectedSocket {
                        ctx: self.as_ctx().clone(),
                        soc: soc,
                    };
                    return Ok((self.into_connected_socket(conn), ep))
                },
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => {
                    if let Err(err) = yield_ctx.wait_for_readable(&self.soc, self.wait_timeout) {
                        return Err(err);
                    }
                }
                Err(OsError::INTERRUPTED) => {
                    if self.ctx.is_stopped() {
                        return Err(OsError::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }
}
