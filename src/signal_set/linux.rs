use super::{Signal, sigaddset, sigdelset, sigemptyset, sigismember, sigmask};
use crate::IoContext;
use crate::error::{OsError, Result};
use crate::primitive::{Fd, Socket, Timeout};
use crate::socket::AsyncSocket;
use std::cell::Cell;
use std::mem::MaybeUninit;
use std::slice;
use std::time::Duration;

fn signalfd_init(set: &libc::sigset_t) -> Result<Fd> {
    sigmask(libc::SIG_BLOCK, set)?;
    unsafe {
        match libc::signalfd(-1, set, libc::SFD_NONBLOCK | libc::SFD_CLOEXEC) {
            -1 => Err(OsError::last()),
            sfd => Ok(Fd::from_raw_fd(sfd)),
        }
    }
}

fn signalfd_update(fd: &Fd, set: &libc::sigset_t) -> Result<()> {
    sigmask(libc::SIG_BLOCK, set)?;
    unsafe {
        match libc::signalfd(fd.as_raw_fd(), set, 0) {
            -1 => Err(OsError::last()),
            _ => Ok(()),
        }
    }
}

fn nb_wait(sfd: &Socket) -> Result<Signal> {
    let mut ssi = MaybeUninit::<libc::signalfd_siginfo>::uninit();
    unsafe {
        let buf = slice::from_raw_parts_mut(
            ssi.as_mut_ptr() as *mut u8,
            size_of::<libc::signalfd_siginfo>(),
        );
        sfd.nb_read(buf)?;
        let ssi = ssi.assume_init();
        Ok(Signal::from_signalfd_siginfo(&ssi))
    }
}

fn wait(sfd: &Socket, ctx: &IoContext, t: Timeout) -> Result<Signal> {
    let mut ssi = MaybeUninit::<libc::signalfd_siginfo>::uninit();
    unsafe {
        let buf = slice::from_raw_parts_mut(
            ssi.as_mut_ptr() as *mut u8,
            size_of::<libc::signalfd_siginfo>(),
        );
        sfd.read(ctx, buf, t)?;
        let ssi = ssi.assume_init();
        Ok(Signal::from_signalfd_siginfo(&ssi))
    }
}

pub struct SignalSet {
    ctx: IoContext,
    sfd: Socket,
    set: Cell<libc::sigset_t>,
    t: Cell<Timeout>,
}

impl SignalSet {
    pub fn new(ctx: &IoContext) -> Result<SignalSet> {
        Self::with_signals(ctx, &[])
    }

    pub fn with_signals(ctx: &IoContext, signals: &[Signal]) -> Result<SignalSet> {
        let mut set = sigemptyset();
        for sig in signals {
            sigaddset(&mut set, *sig);
        }
        let t = ctx.get_timeout();
        let fd = signalfd_init(&set)?;
        Ok(SignalSet {
            ctx: ctx.clone(),
            sfd: Socket(fd),
            set: Cell::new(set),
            t: Cell::new(t),
        })
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn add(&self, sig: Signal) -> Result<bool> {
        let mut set = self.set.get();
        if sigismember(&set, sig) {
            Ok(false)
        } else {
            sigaddset(&mut set, sig);
            signalfd_update(&self.sfd.0, &set)?;
            self.set.set(set);
            Ok(true)
        }
    }

    pub fn del(&self, sig: Signal) -> Result<bool> {
        let mut set = self.set.get();
        if sigismember(&set, sig) {
            sigdelset(&mut set, sig);
            signalfd_update(&self.sfd.0, &set)?;
            self.set.set(set);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn clear(&self) -> Result<()> {
        self.set.set(sigemptyset());
        signalfd_update(&self.sfd.0, &self.set.get())
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.t.set(Timeout::from_duration(timeout))
    }

    pub fn nb_wait(&self) -> Result<Signal> {
        nb_wait(&self.sfd)
    }

    pub fn wait(&self) -> Result<Signal> {
        wait(&self.sfd, &self.ctx, self.t.get())
    }
}

pub struct AsyncSignalSet {
    inner: AsyncSocket<(Cell<Timeout>, Cell<libc::sigset_t>)>,
}

impl AsyncSignalSet {
    pub fn as_ctx(&self) -> &IoContext {
        self.inner.as_ctx()
    }

    pub fn add(&self, sig: Signal) -> Result<bool> {
        let cell = &self.inner.as_data().1;
        let mut set = cell.get();
        if sigismember(&set, sig) {
            Ok(false)
        } else {
            sigaddset(&mut set, sig);
            signalfd_update(&self.inner.as_socket().0, &set)?;
            cell.set(set);
            Ok(true)
        }
    }

    pub fn del(&self, sig: Signal) -> Result<bool> {
        let cell = &self.inner.as_data().1;
        let mut set = cell.get();
        if sigismember(&set, sig) {
            sigdelset(&mut set, sig);
            signalfd_update(&self.inner.as_socket().0, &set)?;
            cell.set(set);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn clear(&self) -> Result<()> {
        let cell = &self.inner.as_data().1;
        let set = sigemptyset();
        signalfd_update(&self.inner.as_socket().0, &set)?;
        cell.set(set);
        Ok(())
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.inner.as_data().0.set(Timeout::from_duration(timeout))
    }

    fn get_timeout(&self) -> Timeout {
        self.inner.as_data().0.get()
    }

    pub fn nb_wait(&self) -> Result<Signal> {
        nb_wait(self.inner.as_socket())
    }

    pub fn wait(&self) -> Result<Signal> {
        let t = self.get_timeout();
        wait(self.inner.as_socket(), self.as_ctx(), t)
    }

    pub async fn async_wait(&self) -> Result<Signal> {
        let t = self.get_timeout();
        let mut ssi = MaybeUninit::<libc::signalfd_siginfo>::uninit();
        unsafe {
            let buf = slice::from_raw_parts_mut(
                ssi.as_mut_ptr() as *mut u8,
                size_of::<libc::signalfd_siginfo>(),
            );
            self.inner.async_read(buf, t).await?;
            let ssi = ssi.assume_init();
            Ok(Signal::from_signalfd_siginfo(&ssi))
        }
    }
}

impl From<SignalSet> for AsyncSignalSet {
    fn from(sfd: SignalSet) -> Self {
        Self {
            inner: AsyncSocket::new(sfd.ctx, sfd.sfd, (sfd.t, sfd.set)),
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
