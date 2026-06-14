use super::{Signal, sigaddset, sigdelset, sigemptyset, sigismember, sigmask};
use crate::IoContext;
use crate::core::AsyncEvent;
use crate::error::{OsError, Result};
use crate::primitive::Timeout;
use crate::socket::AsyncSocket;
use std::cell::Cell;
use std::mem::MaybeUninit;
use std::time::Duration;

fn sigwait(set: &libc::sigset_t) -> Result<Signal> {
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
    pub fn new(ctx: &IoContext) -> Result<SignalSet> {
        Self::with_signals(ctx, &[])
    }

    pub fn with_signals(ctx: &IoContext, signals: &[Signal]) -> Result<SignalSet> {
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

    pub fn add(&mut self, sig: Signal) -> Result<bool> {
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

    pub fn del(&mut self, sig: Signal) -> Result<bool> {
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

    pub fn clear(&mut self) -> Result<()> {
        let set = sigemptyset();
        sigmask(libc::SIG_SETMASK, &set)?;
        self.set.set(set);
        Ok(())
    }

    pub fn wait(&self) -> Result<Signal> {
        sigwait(&self.set.get())
    }
}

pub struct AsyncSignalSet {
    kev: AsyncEvent<(IoContext, Cell<libc::sigset_t>)>,
    t: Timeout,
}

impl AsyncSignalSet {
    pub fn as_ctx(&self) -> &IoContext {
        &self.kev.as_data().0
    }

    pub fn add(&mut self, sig: Signal) -> Result<bool> {
        let cell = &self.kev.as_data().1;
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

    pub fn del(&mut self, sig: Signal) -> Result<bool> {
        let cell = &self.kev.as_data().1;
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

    pub fn clear(&mut self) -> Result<()> {
        let cell = &self.kev.as_data().1;
        let set = sigemptyset();
        sigmask(libc::SIG_SETMASK, &set)?;
        cell.set(set);
        Ok(())
    }

    pub fn wait(&self) -> Result<Signal> {
        let set = self.kev.as_data().1.get();
        sigwait(&set)
    }

    pub async fn async_wait(&self) -> Result<Signal> {
        let event = self.kev.lock();
        match event.poll_sig(self.t).await {
            Ok(sig) => Ok(sig),
            Err(()) => Err(OsError::OPERATION_CANCELED),
        }
    }
}

impl From<SignalSet> for AsyncSignalSet {
    fn from(set: SignalSet) -> Self {
        let kev = AsyncEvent::new((set.ctx, set.set));
        let set = kev.as_data().1.get();
        for sig in Signal::SIGNALS {
            if sigismember(&set, *sig) {
                kev.as_data().0.add_signal(Signal::HUP, &kev);
            }
        }
        Self {
            kev: kev,
            t: Timeout::INFINITE,
        }
    }
}
