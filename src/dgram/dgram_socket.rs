use crate::{ffi, Error, IoContext, Protocol, Result, Shutdown};
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

    pub fn connect(self, ep: &P::Endpoint) -> Result<DgramSocket<P>> {
        let soc = ffi::socket(self.pro)?;
        ffi::connect(&soc, ep)?;
        Ok(DgramSocket::new_priv(self.ctx, self.pro, soc))
    }

    pub fn listen(self) -> Result<DgramSocket<P>> {
        let soc = ffi::socket(self.pro)?;
        if let Some(ep) = self.ep {
            ffi::bind(&soc, &ep)?;
        }
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

    fn new_priv(ctx: IoContext, pro: P, soc: OwnedFd) -> Self {
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

    pub fn send(&self, buf: &[u8]) -> Result<usize> {
        loop {
            match ffi::send(&self.soc, buf) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(Error::TRY_AGAIN) | Err(Error::WOULD_BLOCK) => {
                    if let Err(err) = ffi::wait_writable(&self.soc, self.write_timeout) {
                        return Err(err);
                    }
                }
                Err(Error::INTERRUPTED) => {
                    if self.ctx.is_stopped() {
                        return Err(Error::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub fn send_to(&self, buf: &[u8], ep: &P::Endpoint) -> Result<usize> {
        loop {
            match ffi::send_to(&self.soc, buf, ep) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(Error::TRY_AGAIN) | Err(Error::WOULD_BLOCK) => {
                    if let Err(err) = ffi::wait_writable(&self.soc, self.write_timeout) {
                        return Err(err);
                    }
                }
                Err(Error::INTERRUPTED) => {
                    if self.ctx.is_stopped() {
                        return Err(Error::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize> {
        loop {
            match ffi::receive(&self.soc, buf) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(Error::TRY_AGAIN) | Err(Error::WOULD_BLOCK) => {
                    if let Err(err) = ffi::wait_readable(&self.soc, self.read_timeout) {
                        return Err(err);
                    }
                }
                Err(Error::INTERRUPTED) => {
                    if self.ctx.is_stopped() {
                        return Err(Error::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub fn receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint)> {
        loop {
            match ffi::receive_from(&self.soc, buf) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(Error::TRY_AGAIN) | Err(Error::WOULD_BLOCK) => {
                    if let Err(err) = ffi::wait_readable(&self.soc, self.read_timeout) {
                        return Err(err);
                    }
                }
                Err(Error::INTERRUPTED) => {
                    if self.ctx.is_stopped() {
                        return Err(Error::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize> {
        ffi::receive(&self.soc, buf)
    }

    pub fn nb_receive_from(&self, buf: &mut [u8]) -> Result<(usize, P::Endpoint)> {
        ffi::receive_from(&self.soc, buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        ffi::send(&self.soc, buf)
    }

    pub fn nb_send_to(&self, buf: &[u8], ep: &P::Endpoint) -> Result<usize> {
        ffi::send_to(&self.soc, buf, ep)
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint> {
        ffi::getsockname(&self.soc)
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint> {
        ffi::getpeername(&self.soc)
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        ffi::shutdown(&self.soc, how)
    }

    pub fn close(self) -> Result<()> {
        ffi::close(self.soc)
    }
}
