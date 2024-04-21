use crate::{ffi, Error, Result, Signal};
use std::os::fd::OwnedFd;

pub struct SignalSetBuilder {
    set: libc::sigset_t,
    err: Option<Error>,
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

    pub fn ready(self) -> Result<SignalSet> {
        if let Some(err) = self.err {
            return Err(err);
        }

        let _ = ffi::sigprocmask(libc::SIG_BLOCK, &self.set)?;
        Ok(SignalSet {
            sfd: ffi::signalfd(&self.set)?,
        })
    }
}

pub struct SignalSet {
    sfd: OwnedFd,
}

impl SignalSet {
    pub fn new() -> SignalSetBuilder {
        SignalSetBuilder {
            set: ffi::sigemptyset(),
            err: None,
        }
    }

    pub fn nb_signal(&self) -> Result<Signal> {
        ffi::signalfd_read(&self.sfd)
    }
}
