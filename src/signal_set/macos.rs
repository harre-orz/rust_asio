use super::{Signal, sigaddset, sigdelset, sigemptyset, sigismember, sigmask};
use crate::IoContext;
use crate::core::AsyncEvent;
use crate::error::OsError;
use crate::primitive::AtomicTimeout;
use std::cell::Cell;
use std::mem::MaybeUninit;

fn sigwait(set: &libc::sigset_t) -> Result<Signal, OsError> {
    let mut sig = MaybeUninit::uninit();
    unsafe {
        match libc::sigwait(set, sig.as_mut_ptr()) {
            0 => Ok(Signal::from_raw(sig.assume_init())),
            _ => Err(OsError::last()),
        }
    }
}

pub struct SignalSet {
    ctx: IoContext,
    set: Cell<libc::sigset_t>,
}

impl SignalSet {
    pub fn new(ctx: &IoContext) -> Result<SignalSet, OsError> {
        Self::with_signals(ctx, &[])
    }

    pub fn with_signals(ctx: &IoContext, signals: &[Signal]) -> Result<SignalSet, OsError> {
        let mut set = sigemptyset();
        for sig in signals.as_ref() {
            sigaddset(&mut set, *sig);
        }
        sigmask(libc::SIG_SETMASK, &set)?;
        Ok(SignalSet {
            ctx: ctx.clone(),
            set: Cell::new(set),
        })
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn add(&mut self, sig: Signal) -> Result<bool, OsError> {
        let mut set = self.set.get();
        if sigismember(&set, sig) {
            Ok(false)
        } else {
            sigaddset(&mut set, sig);
            sigmask(libc::SIG_SETMASK, &set)?;
            self.set.set(set);
            Ok(true)
        }
    }

    pub fn del(&mut self, sig: Signal) -> Result<bool, OsError> {
        let mut set = self.set.get();
        if sigismember(&set, sig) {
            sigdelset(&mut set, sig);
            sigmask(libc::SIG_SETMASK, &set)?;
            self.set.set(set);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn clear(&mut self) -> Result<(), OsError> {
        let set = sigemptyset();
        sigmask(libc::SIG_SETMASK, &set)?;
        self.set.set(set);
        Ok(())
    }

    pub fn wait(&self) -> Result<Signal, OsError> {
        sigwait(&self.set.get())
    }
}

pub struct AsyncSignalSet {
    inner: AsyncEvent<(IoContext, Cell<libc::sigset_t>, AtomicTimeout)>,
}

impl AsyncSignalSet {
    pub fn as_ctx(&self) -> &IoContext {
        &self.inner.as_data().0
    }

    pub fn add(&mut self, sig: Signal) -> Result<bool, OsError> {
        let cell = &self.inner.as_data().1;
        let mut set = cell.get();
        if sigismember(&set, sig) {
            Ok(false)
        } else {
            sigaddset(&mut set, sig);
            sigmask(libc::SIG_SETMASK, &set)?;
            cell.set(set);
            Ok(true)
        }
    }

    pub fn del(&mut self, sig: Signal) -> Result<bool, OsError> {
        let cell = &self.inner.as_data().1;
        let mut set = cell.get();
        if sigismember(&set, sig) {
            sigdelset(&mut set, sig);
            sigmask(libc::SIG_SETMASK, &set)?;
            cell.set(set);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn clear(&mut self) -> Result<(), OsError> {
        let cell = &self.inner.as_data().1;
        let set = sigemptyset();
        sigmask(libc::SIG_SETMASK, &set)?;
        cell.set(set);
        Ok(())
    }

    pub fn wait(&self) -> Result<Signal, OsError> {
        let set = self.inner.as_data().1.get();
        sigwait(&set)
    }

    pub async fn async_wait(&self) -> Result<Signal, OsError> {
        let event = self.inner.lock(self.as_ctx());
        match event.poll_sig(self.inner.as_data().2.get()).await {
            Ok(sig) => Ok(sig),
            Err(()) => Err(OsError::OPERATION_CANCELED),
        }
    }
}

impl From<SignalSet> for AsyncSignalSet {
    fn from(set: SignalSet) -> Self {
        let ato = set.ctx.timeout();
        let inner = AsyncEvent::new((set.ctx, set.set, ato));
        let set = inner.as_data().1.get();
        for sig in Signal::SIGNALS {
            if sigismember(&set, *sig) {
                inner.as_data().0.add_signal(Signal::HUP, &inner);
            }
        }
        Self { inner: inner }
    }
}
