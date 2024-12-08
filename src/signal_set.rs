use crate::error::OsError;
use crate::executor::{AsyncSocket, IoContext};
use crate::ffi::{self, Socket};
use crate::ops;
use std::time::{Duration, Instant};
use std::cell::Cell;

pub use crate::ffi::Signal;

pub struct SignalSetBuilder<'a> {
    ctx: &'a IoContext,
    set: libc::sigset_t,
}

impl<'a> SignalSetBuilder<'a> {
    pub fn add(mut self, signal: Signal) -> Result<Self, OsError> {
        ffi::sigaddset(&mut self.set, signal)?;
        Ok(self)
    }

    pub fn any(mut self) -> Self {
        self.set = ffi::sigfillset();
        self
    }

    pub fn listen(self) -> Result<SignalSet, OsError> {
        ffi::sigprocmask(libc::SIG_BLOCK, &self.set)?;
        let sfd = ffi::signalfd(&self.set)?;
        Ok(SignalSet {
            ctx: self.ctx.clone(),
            sfd: sfd,
	    exp: Cell::new(None),
        })
    }

    pub fn listen_async(self) -> Result<AsyncSignalSet, OsError> {
        let ss = self.listen()?;
        Ok(ss.into())
    }
}

pub struct SignalSet {
    ctx: IoContext,
    sfd: Socket,
    exp: Cell<Option<Instant>>,
}

impl SignalSet {
    pub fn new(ctx: &IoContext) -> SignalSetBuilder {
        SignalSetBuilder {
            ctx: ctx,
            set: ffi::sigemptyset(),
        }
    }

    pub fn close(self) -> Result<(), OsError> {
        self.sfd.close()
    }

    pub fn expires_at(&self, time: Instant) {
	self.exp.set(Some(time))
    }

    pub fn expires_from_now(&self, time: Duration) {
	self.exp.set(Some(Instant::now() + time))
    }

    pub fn nb_wait(&self) -> Result<Signal, OsError> {
        ffi::signal_read(&self.sfd)
    }

    pub fn wait(&self) -> Result<Signal, OsError> {
        ops::signal_read(&self.sfd, &self.ctx, self.exp.get())
    }
}

pub struct AsyncSignalSet {
    sfd: AsyncSocket,
}

impl AsyncSignalSet {
    pub fn expires_at(&self, time: Instant) {
	self.sfd.expires_at(time)
    }

    pub fn expires_from_now(&self, time: Duration) {
	self.sfd.expires_at(Instant::now() + time)
    }
    
    pub fn nb_wait(&self) -> Result<Signal, OsError> {
        ffi::signal_read(self.sfd.as_socket())
    }

    pub async fn async_wait(&self) -> Result<Signal, OsError> {
        ops::async_signal_read(&self.sfd).await
    }
}

impl From<SignalSet> for AsyncSignalSet {
    fn from(sfd: SignalSet) -> AsyncSignalSet {
        Self {
            sfd: AsyncSocket::new(sfd.ctx, sfd.sfd),
        }
    }
}
