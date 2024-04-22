use crate::{ffi, ops, IoContext, OsError, Protocol, Shutdown};
use std::os::fd::OwnedFd;
use std::time::Duration;

pub struct DgramSocketBuilder<P: Protocol> {
    ctx: IoContext,
    pro: P,
    ep: Option<P::Endpoint>,
}

impl<P> DgramSocketBuilder<P>
where
    P: Protocol,
{
    pub fn bind(mut self, ep: P::Endpoint) -> Self {
        self.ep = Some(ep);
        self
    }

    pub fn listen(self) -> Result<DgramSocket<P>, OsError> {
        let soc = ffi::socket(self.pro)?;
        if let Some(ep) = self.ep {
            ffi::bind(&soc, &ep)?;
        }
        Ok(DgramSocket::new_priv(self.ctx, self.pro, soc))
    }

    pub fn connect(self, ep: &P::Endpoint) -> Result<DgramSocket<P>, OsError> {
        let soc = ffi::socket(self.pro)?;
        if let Some(ep) = self.ep {
            ffi::bind(&soc, &ep)?;
        }
        ffi::connect(&soc, ep)?;
        Ok(DgramSocket::new_priv(self.ctx, self.pro, soc))
    }
}

pub struct DgramSocket<P> {
    ctx: IoContext,
    pro: P,
    soc: OwnedFd,
    read_timeout: Duration,
    write_timeout: Duration,
}

impl<P> DgramSocket<P>
where
    P: Protocol,
{
    pub fn new(ctx: &IoContext, pro: P) -> DgramSocketBuilder<P> {
        DgramSocketBuilder {
            ctx: ctx.clone(),
            pro: pro,
            ep: None,
        }
    }

    pub(crate) fn new_priv(ctx: IoContext, pro: P, soc: OwnedFd) -> Self {
        Self {
            ctx: ctx,
            pro: pro,
            soc: soc,
            read_timeout: Duration::MAX,
            write_timeout: Duration::MAX,
        }
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn protocol(&self) -> P {
        self.pro
    }

    pub fn close(self) -> Result<(), OsError> {
        ffi::close(self.soc)
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<(), OsError> {
        ffi::shutdown(&self.soc, how)
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getsockname(&self.soc)
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getpeername(&self.soc)
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ffi::receive(&self.soc, buf)
    }

    pub fn nb_receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint), OsError> {
        ffi::receive_from(&self.soc, buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ffi::send(&self.soc, buf)
    }

    pub fn nb_send_to(&self, buf: &[u8], ep: &P::Endpoint) -> Result<usize, OsError> {
        ffi::send_to(&self.soc, buf, ep)
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::send(&self.ctx, &self.soc, buf, self.read_timeout)
    }

    pub async fn async_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ops::async_send(&self.ctx, &self.soc, buf, self.read_timeout).await
    }

    pub fn send_to(&self, buf: &[u8], ep: &P::Endpoint) -> Result<usize, OsError> {
        ops::send_to(&self.ctx, &self.soc, buf, ep, self.read_timeout)
    }

    pub async fn async_send_to(&self, buf: &[u8], ep: &P::Endpoint) -> Result<usize, OsError> {
        ops::async_send_to(&self.ctx, &self.soc, buf, ep, self.read_timeout).await
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::receive(&self.ctx, &self.soc, buf, self.read_timeout)
    }

    pub async fn async_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ops::async_receive(&self.ctx, &self.soc, buf, self.read_timeout).await
    }

    pub fn receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint), OsError> {
        ops::receive_from(&self.ctx, &self.soc, buf, self.read_timeout)
    }

    pub async fn async_receive_from(
        &self,
        buf: &mut [u8],
    ) -> Result<(usize, P::Endpoint), OsError> {
        ops::async_receive_from(&self.ctx, &self.soc, buf, self.read_timeout).await
    }
}
