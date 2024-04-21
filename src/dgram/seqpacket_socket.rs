use crate::{ffi, OsError, IoContext, Protocol, Shutdown, YieldContext};
use std::os::fd::OwnedFd;
use std::time::Duration;

pub struct SeqPacketSocketBuilder<P: Protocol>
{
    ctx: IoContext,
    pro: P,
    ep: Option<P::Endpoint>,
}

impl<P> SeqPacketSocketBuilder<P>
where
    P: Protocol,
{
    pub fn bind(mut self, ep: P::Endpoint) -> Self {
        self.ep = Some(ep);
        self
    }

    pub fn listen(self) -> Result<SeqPacketSocket<P>, OsError> {
        let soc = ffi::socket(self.pro)?;
        if let Some(ep) = self.ep {
            ffi::bind(&soc, &ep)?;
        }
        Ok(SeqPacketSocket::new_priv(self.ctx, self.pro, soc))
    }

    pub fn connect(self, ep: &P::Endpoint) -> Result<SeqPacketSocket<P>, OsError> {
        let soc = ffi::socket(self.pro)?;
        if let Some(ep) = self.ep {
            ffi::bind(&soc, &ep)?;
        }
        ffi::connect(&soc, ep)?;
        Ok(SeqPacketSocket::new_priv(self.ctx, self.pro, soc))
    }
}

pub struct SeqPacketSocket<P> {
    ctx: IoContext,
    pro: P,
    soc: OwnedFd,
    read_timeout: Duration,
    write_timeout: Duration,
}

impl<P> SeqPacketSocket<P>
where
    P: Protocol,
{
    pub fn new(ctx: &IoContext, pro: P) -> SeqPacketSocketBuilder<P> {
        SeqPacketSocketBuilder {
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

    pub fn send(&self, buf: &[u8], yield_ctx: &mut YieldContext) -> Result<usize, OsError> {
        loop {
            match ffi::send(&self.soc, buf) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => {
                    if let Err(err) = yield_ctx.wait_for_writable(&self.soc, self.write_timeout) {
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

    pub fn receive(&self, buf: &mut [u8], yield_ctx: &mut YieldContext) -> Result<usize, OsError> {
        loop {
            match ffi::receive(&self.soc, buf) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => {
                    if let Err(err) = yield_ctx.wait_for_readable(&self.soc, self.read_timeout) {
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

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize, OsError> {
        ffi::receive(&self.soc, buf)
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize, OsError> {
        ffi::send(&self.soc, buf)
    }

    pub fn local_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getsockname(&self.soc)
    }

    pub fn remote_endpoint(&self) -> Result<P::Endpoint, OsError> {
        ffi::getpeername(&self.soc)
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<(), OsError> {
        ffi::shutdown(&self.soc, how)
    }

    pub fn close(self) -> Result<(), OsError> {
        ffi::close(self.soc)
    }
}
