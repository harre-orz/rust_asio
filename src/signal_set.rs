use crate::error::OsError;
use crate::ffi::{self, ConnectedSocket};
use crate::ops;
use crate::IoContext;
use std::time::Duration;

pub use crate::ffi::Signal;

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
    sfd: ConnectedSocket,
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

    fn new_priv(ctx: IoContext, sfd: ConnectedSocket) -> Self {
        Self {
            ctx: ctx,
            sfd: sfd,
            wait_timeout: Duration::MAX,
        }
    }

    pub async fn async_signal_read(&self) -> Result<Signal, OsError> {
        ops::async_signal_read(&self.ctx, &self.sfd, self.wait_timeout).await
    }

    pub fn close(self) -> Result<(), OsError> {
        self.sfd.close()
    }

    pub fn nb_signal_read(&self) -> Result<Signal, OsError> {
        ffi::signal_read(&self.sfd)
    }

    pub fn signal_read(&self) -> Result<Signal, OsError> {
        ops::signal_read(&self.ctx, &self.sfd, self.wait_timeout)
    }
}
