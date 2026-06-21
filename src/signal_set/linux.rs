use super::{Signal, sigaddset, sigdelset, sigemptyset, sigismember, sigmask};
use crate::IoContext;
use crate::error::OsError;
use crate::primitive::{AtomicTimeout, Fd, Socket, Timeout, TimeoutError};
use crate::socket::AsyncSocket;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::slice;
use std::time::Duration;

fn signalfd_init(set: &libc::sigset_t) -> Result<Fd, OsError> {
    sigmask(libc::SIG_BLOCK, set)?;
    unsafe {
        match libc::signalfd(-1, set, libc::SFD_NONBLOCK | libc::SFD_CLOEXEC) {
            -1 => Err(OsError::last()),
            sfd => Ok(Fd::from_raw_fd(sfd)),
        }
    }
}

fn signalfd_update(fd: &Fd, set: &libc::sigset_t) -> Result<(), OsError> {
    sigmask(libc::SIG_BLOCK, set)?;
    unsafe {
        match libc::signalfd(fd.as_raw_fd(), set, 0) {
            -1 => Err(OsError::last()),
            _ => Ok(()),
        }
    }
}

fn nb_wait(sfd: &Socket) -> Result<Signal, OsError> {
    let mut ssi = MaybeUninit::<libc::signalfd_siginfo>::uninit();
    unsafe {
        let buf = slice::from_raw_parts_mut(
            ssi.as_mut_ptr() as *mut u8,
            size_of::<libc::signalfd_siginfo>(),
        );
        sfd.nb_read(buf)?;
        let ssi = ssi.assume_init();
        Ok(Signal::from_ssi(ssi))
    }
}

fn wait(sfd: &Socket, ctx: &IoContext, t: Timeout) -> Result<Signal, OsError> {
    let mut ssi = MaybeUninit::<libc::signalfd_siginfo>::uninit();
    unsafe {
        let buf = slice::from_raw_parts_mut(
            ssi.as_mut_ptr() as *mut u8,
            size_of::<libc::signalfd_siginfo>(),
        );
        sfd.read(ctx, buf, t)?;
        let ssi = ssi.assume_init();
        Ok(Signal::from_ssi(ssi))
    }
}

pub struct SignalSet {
    ctx: IoContext,
    sfd: Socket,
    ato: AtomicTimeout,
    set: UnsafeCell<libc::sigset_t>,
}

impl SignalSet {
    pub fn new(ctx: &IoContext) -> Result<SignalSet, OsError> {
        Self::with_signals(ctx, &[])
    }

    pub fn with_signals(ctx: &IoContext, signals: &[Signal]) -> Result<SignalSet, OsError> {
        let mut set = sigemptyset();
        for sig in signals {
            sigaddset(&mut set, *sig);
        }
        let fd = signalfd_init(&set)?;
        let ato = ctx.timeout();
        Ok(SignalSet {
            ctx: ctx.clone(),
            sfd: Socket(fd),
            ato: ato,
            set: UnsafeCell::new(set),
        })
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn add(&self, sig: Signal) -> Result<bool, OsError> {
        let set = unsafe { &mut *self.set.get() };
        if sigismember(set, sig) {
            Ok(false)
        } else {
            sigaddset(set, sig);
            signalfd_update(&self.sfd.0, set)?;
            Ok(true)
        }
    }

    pub fn del(&self, sig: Signal) -> Result<bool, OsError> {
        let set = unsafe { &mut *self.set.get() };
        if sigismember(set, sig) {
            sigdelset(set, sig);
            signalfd_update(&self.sfd.0, &set)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn clear(&self) -> Result<(), OsError> {
        let set = unsafe { &mut *self.set.get() };
        *set = sigemptyset();
        signalfd_update(&self.sfd.0, set)
    }

    pub fn set_timeout(&mut self, timer: Duration) -> Result<(), TimeoutError> {
        self.ato.set(timer)
    }

    pub fn nb_wait(&self) -> Result<Signal, OsError> {
        nb_wait(&self.sfd)
    }

    pub fn wait(&self) -> Result<Signal, OsError> {
        wait(&self.sfd, &self.ctx, self.ato.get())
    }
}

pub struct AsyncSignalSet {
    inner: AsyncSocket<UnsafeCell<libc::sigset_t>>,
}

impl AsyncSignalSet {
    pub fn as_ctx(&self) -> &IoContext {
        &self.inner.0.1.0
    }

    pub fn add(&self, sig: Signal) -> Result<bool, OsError> {
        let (_, soc, _, set) = &self.inner.0.1;
        let set = unsafe { &mut *set.get() };
        if sigismember(set, sig) {
            Ok(false)
        } else {
            sigaddset(set, sig);
            signalfd_update(&soc.0, &set)?;
            Ok(true)
        }
    }

    pub fn del(&self, sig: Signal) -> Result<bool, OsError> {
        let (_, soc, _, set) = &self.inner.0.1;
        let set = unsafe { &mut *set.get() };
        if sigismember(set, sig) {
            sigdelset(set, sig);
            signalfd_update(&soc.0, &set)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn clear(&self) -> Result<(), OsError> {
        let (_, soc, _, set) = &self.inner.0.1;
        let set = unsafe { &mut *set.get() };
        signalfd_update(&soc.0, &set)?;
        Ok(())
    }

    pub fn set_timeout(&mut self, timeout: Duration) -> Result<(), TimeoutError> {
        self.inner.0.1.2.set(timeout)
    }

    pub fn nb_wait(&self) -> Result<Signal, OsError> {
        nb_wait(&self.inner.0.1.1)
    }

    pub fn wait(&self) -> Result<Signal, OsError> {
        let (ctx, soc, ato, _) = &self.inner.0.1;
        wait(soc, ctx, ato.get())
    }

    pub async fn async_wait(&self) -> Result<Signal, OsError> {
        let mut ssi = MaybeUninit::<libc::signalfd_siginfo>::uninit();
        unsafe {
            let buf = slice::from_raw_parts_mut(
                ssi.as_mut_ptr() as *mut u8,
                size_of::<libc::signalfd_siginfo>(),
            );
            self.inner.async_read(buf).await?;
            let ssi = ssi.assume_init();
            Ok(Signal::from_ssi(ssi))
        }
    }
}

impl From<SignalSet> for AsyncSignalSet {
    fn from(sfd: SignalSet) -> Self {
        Self {
            inner: AsyncSocket::new(sfd.ctx, sfd.sfd, sfd.ato, sfd.set),
        }
    }
}

#[test]
fn test_signal() {
    let ctx = &IoContext::new().unwrap();
    let sig = SignalSet::new(ctx).unwrap();

    assert_eq!(sig.add(Signal::HUP), Ok(true));
    assert_eq!(sig.add(Signal::HUP), Ok(false));
}
