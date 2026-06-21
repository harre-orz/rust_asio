use super::{Signal, sigaddset, sigdelset, sigemptyset, sigismember, sigmask};
use crate::core::{Event, IoContext};
use crate::error::OsError;
use crate::primitive::AtomicTimeout;
use std::cell::{Cell, UnsafeCell};
use std::mem::MaybeUninit;
use std::pin::Pin;

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
    set: UnsafeCell<libc::sigset_t>,
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
            set: UnsafeCell::new(set),
        })
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn add(&mut self, sig: Signal) -> Result<bool, OsError> {
        let mut set = unsafe { &mut *self.set.get() };
        if sigismember(&set, sig) {
            Ok(false)
        } else {
            sigaddset(&mut set, sig);
            sigmask(libc::SIG_SETMASK, &set)?;
            Ok(true)
        }
    }

    pub fn del(&mut self, sig: Signal) -> Result<bool, OsError> {
        let mut set = unsafe { &mut *self.set.get() };
        if sigismember(&set, sig) {
            sigdelset(&mut set, sig);
            sigmask(libc::SIG_SETMASK, &set)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn clear(&mut self) -> Result<(), OsError> {
        let set = unsafe { &mut *self.set.get() };
        *set = sigemptyset();
        sigmask(libc::SIG_SETMASK, &set)?;
        Ok(())
    }

    pub fn wait(&self) -> Result<Signal, OsError> {
        sigwait(unsafe { &*self.set.get() })
    }
}

pub struct AsyncSignalSet {
    inner: Pin<
        Box<(
            Event,
            (IoContext, UnsafeCell<libc::sigset_t>, AtomicTimeout),
        )>,
    >,
}

impl AsyncSignalSet {
    pub fn as_ctx(&self) -> &IoContext {
        &self.inner.1.0
    }

    pub fn add(&mut self, sig: Signal) -> Result<bool, OsError> {
        let (event, (ctx, set, _)) = &*self.inner;
        let set = unsafe { &mut *set.get() };
        if sigismember(set, sig) {
            Ok(false)
        } else {
            sigaddset(set, sig);
            sigmask(libc::SIG_SETMASK, set)?;
            ctx.inner.reactor.add_signal(sig, event);
            Ok(true)
        }
    }

    pub fn del(&mut self, sig: Signal) -> Result<bool, OsError> {
        let (event, (ctx, set, _)) = &*self.inner;
        let set = unsafe { &mut *set.get() };
        if sigismember(set, sig) {
            sigaddset(set, sig);
            sigmask(libc::SIG_SETMASK, set)?;
            ctx.inner.reactor.del_signal(sig);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn clear(&mut self) -> Result<(), OsError> {
        let (event, (ctx, set, _)) = &*self.inner;
        let set = unsafe { &mut *set.get() };
        for sig in Signal::ALL {
            let sig = *sig;
            if sigismember(set, sig) {
                ctx.inner.reactor.del_signal(sig);
            }
        }
        *set = sigemptyset();
        sigmask(libc::SIG_SETMASK, set)?;
        Ok(())
    }

    pub fn wait(&self) -> Result<Signal, OsError> {
        let set = unsafe { &*self.inner.1.1.get() };
        sigwait(set)
    }

    pub async fn async_wait(&self) -> Result<Signal, OsError> {
        let (event, (ctx, _, ato)) = &*self.inner;
        match ctx.lock(event).poll_sig(ato.get()).await {
            Ok(sig) => Ok(sig),
            Err(()) => Err(OsError::OPERATION_CANCELED),
        }
    }
}

impl From<SignalSet> for AsyncSignalSet {
    fn from(set: SignalSet) -> Self {
        let ato = set.ctx.timeout();
        let SignalSet { ctx, set } = set;
        let inner = Event::new((ctx, set, ato));
        let (event, (ctx, set, ato)) = &*inner;
        let set = unsafe { &*set.get() };
        for sig in Signal::ALL {
            let sig = *sig;
            if sigismember(set, sig) {
                ctx.inner.reactor.add_signal(sig, &inner.0);
            }
        }
        Self { inner: inner }
    }
}
