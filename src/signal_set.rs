use crate::error::OsError;
use crate::executor::{AsyncSocket, IoContext};
use crate::ffi::{self, ConnectedSocket, Timeout};
use crate::ops;

pub use crate::ffi::Signal;

pub struct SignalSetBuilder<'a> {
    ctx: &'a IoContext,
    set: libc::sigset_t,
}

impl<'a> SignalSetBuilder<'a> {
    pub fn new(ctx: &'a IoContext) -> SignalSetBuilder {
        Self {
            ctx: ctx,
            set: ffi::sigemptyset(),
        }
    }

    pub fn add(mut self, signal: Signal) -> Result<Self, OsError> {
        ffi::sigaddset(&mut self.set, signal)?;
        Ok(self)
    }

    pub fn any(mut self) -> Self {
        self.set = ffi::sigfillset();
        self
    }

    pub fn ready(self) -> Result<SignalSet, OsError> {
        ffi::sigprocmask(libc::SIG_BLOCK, &self.set)?;
        let sfd = ffi::signalfd(&self.set)?;
        Ok(SignalSet::new_priv(self.ctx, sfd))
    }
}

pub struct SignalSet {
    ctx: IoContext,
    sfd: ConnectedSocket,
    read_timeout: Timeout,
}

impl SignalSet {
    fn new_priv(ctx: &IoContext, sfd: ConnectedSocket) -> Self {
        Self {
            ctx: ctx.clone(),
            sfd: sfd,
            read_timeout: Timeout::new(),
        }
    }

    pub fn close(self) -> Result<(), OsError> {
        self.sfd.close()
    }

    pub fn nb_signal_read(&self) -> Result<Signal, OsError> {
        ffi::signal_read(&self.sfd)
    }

    pub fn signal_read(&self) -> Result<Signal, OsError> {
        ops::signal_read(&self.sfd, self.read_timeout, &self.ctx)
    }
}

pub struct AsyncSignalSet {
    sfd: AsyncSocket,
    read_timeout: Timeout,
}

impl AsyncSignalSet {
    pub fn nb_signal_read(&self) -> Result<Signal, OsError> {
        ffi::signal_read(self.sfd.as_socket())
    }

    pub fn signal_read(&self) -> Result<Signal, OsError> {
        ops::signal_read(self.sfd.as_socket(), self.read_timeout, self.sfd.as_ctx())
    }

    pub async fn async_signal_read(&self) -> Result<Signal, OsError> {
        ops::async_signal_read(&self.sfd, self.read_timeout).await
    }
}

impl From<SignalSet> for AsyncSignalSet {
    fn from(sfd: SignalSet) -> AsyncSignalSet {
        let SignalSet {
            ctx,
            sfd,
            read_timeout,
        } = sfd;
        Self {
            sfd: AsyncSocket::new(ctx, sfd),
            read_timeout: read_timeout,
        }
    }
}
