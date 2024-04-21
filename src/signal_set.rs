use crate::{ffi, IoContext, OsError, Signal, YieldContext};
use std::os::fd::OwnedFd;
use std::time::Duration;

pub struct SignalSetBuilder {
    ctx: IoContext,
    set: libc::sigset_t,
    err: Option<OsError>,
}

impl SignalSetBuilder {
    pub fn add(mut self, signal: Signal) -> Self {
        if self.err.is_none() {
            if let Err(err) = ffi::sigaddset(&mut self.set, signal) {
                self.err = Some(err);
            }
        }
        self
    }

    pub fn any(mut self) -> Self {
        if self.err.is_none() {
            self.set = ffi::sigfillset();
        }
        self
    }

    pub fn ready(self) -> Result<SignalSet, OsError> {
        if let Some(err) = self.err {
            return Err(err);
        }

        let _ = ffi::sigprocmask(libc::SIG_BLOCK, &self.set)?;
        let sfd = ffi::signalfd(&self.set)?;
        Ok(SignalSet::new_priv(self.ctx, sfd))
    }
}

pub struct SignalSet {
    ctx: IoContext,
    sfd: OwnedFd,
    wait_timeout: Duration,
}

impl SignalSet {
    pub fn new(ctx: &IoContext) -> SignalSetBuilder {
        SignalSetBuilder {
            ctx: ctx.clone(),
            set: ffi::sigemptyset(),
            err: None,
        }
    }

    fn new_priv(ctx: IoContext, sfd: OwnedFd) -> Self {
        Self {
            ctx: ctx,
            sfd: sfd,
            wait_timeout: Duration::MAX,
        }
    }

    pub fn nb_signal(&self) -> Result<Signal, OsError> {
        ffi::signalfd_read(&self.sfd)

    }

    pub fn signal(&self, yield_ctx: &mut YieldContext) -> Result<Signal, OsError> {
        loop {
            match ffi::signalfd_read(&self.sfd) {
                Ok(len) => return Ok(len),
                #[allow(unreachable_patterns)]
                Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => {
                    if let Err(err) = yield_ctx.wait_for_readable(&self.sfd, self.wait_timeout) {
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
