use crate::error::OsError;
use crate::executor::{AsyncSocket, IoContext};
use crate::ffi::{self, ConnectedSocket};
use crate::ops;
use std::time::Duration;

pub use crate::ffi::Signal;

pub struct SignalSetBuilder<'a> {
    ctx: &'a IoContext,
    set: libc::sigset_t,
    err: Option<OsError>,
}

impl<'a> SignalSetBuilder<'a> {
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

    pub fn async_ready(self) -> Result<AsyncSignalSet, OsError> {
        if let Some(err) = self.err {
            return Err(err);
        }

        let _ = ffi::sigprocmask(libc::SIG_BLOCK, &self.set)?;
        let sfd = ffi::signalfd(&self.set)?;
        Ok(AsyncSignalSet::new_priv(self.ctx, sfd))
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
            ctx: ctx,
            set: ffi::sigemptyset(),
            err: None,
        }
    }

    fn new_priv(ctx: &IoContext, sfd: ConnectedSocket) -> Self {
        Self {
            ctx: ctx.clone(),
            sfd: sfd,
            wait_timeout: Duration::MAX,
        }
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

pub struct AsyncSignalSet {
    sfd: AsyncSocket,
    wait_timeout: Duration,
}

impl AsyncSignalSet {
    fn new_priv(ctx: &IoContext, sfd: ConnectedSocket) -> Self {
        Self {
            sfd: ctx.async_socket(sfd),
            wait_timeout: Duration::MAX,
        }
    }

    pub fn nb_signal_read(&self) -> Result<Signal, OsError> {
        ffi::signal_read(self.sfd.as_socket())
    }

    pub fn signal_read(&self) -> Result<Signal, OsError> {
        ops::signal_read(self.sfd.as_ctx(), self.sfd.as_socket(), self.wait_timeout)
    }

    pub async fn async_signal_read(&self) -> Result<Signal, OsError> {
        ops::async_signal_read(&self.sfd, self.wait_timeout).await
    }
}
