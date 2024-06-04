use crate::error::OsError;
use crate::executor::{AsyncSocket, IoContext};
use crate::ffi::{self, Socket, Timeout};
use crate::ops;

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
            sfd,
            read_timeout: Timeout::new(),
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
    read_timeout: Timeout,
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

    pub fn nb_wait(&self) -> Result<Signal, OsError> {
        ffi::signal_read(&self.sfd)
    }

    pub fn wait(&self) -> Result<Signal, OsError> {
        ops::signal_read(&self.sfd, self.read_timeout, &self.ctx)
    }
}

pub struct AsyncSignalSet {
    sfd: AsyncSocket,
    read_timeout: Timeout,
}

impl AsyncSignalSet {
    pub fn nb_wait(&self) -> Result<Signal, OsError> {
        ffi::signal_read(self.sfd.as_socket())
    }

    pub fn wait(&self) -> Result<Signal, OsError> {
        ops::signal_read(self.sfd.as_socket(), self.read_timeout, self.sfd.as_ctx())
    }

    pub async fn async_wait(&self) -> Result<Signal, OsError> {
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
