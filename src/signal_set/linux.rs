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
    t: Timeout,
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
        let fd = signalfd_init(&set)?;
        Ok(SignalSet {
            ctx: ctx.clone(),
            sfd: Socket(fd),
            set: Cell::new(set),
            t: Timeout::INFINITE,
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

    pub const fn set_timeout(&mut self, timeout: Duration) {
        self.t = Timeout::from_duration(timeout)
    }

    pub fn nb_wait(&self) -> Result<Signal> {
        nb_wait(&self.sfd)
    }

    pub fn wait(&self) -> Result<Signal> {
        wait(&self.sfd, &self.ctx, self.t)
    }
}

pub struct AsyncSignalSet {
    sfd: AsyncSocket<Cell<libc::sigset_t>>,
    t: Timeout,
}

impl AsyncSignalSet {
    pub fn as_ctx(&self) -> &IoContext {
        self.sfd.as_ctx()
    }

    pub fn add(&self, sig: Signal) -> Result<bool> {
        let cell = self.sfd.as_data();
        let mut set = cell.get();
        if sigismember(&set, sig) {
            Ok(false)
        } else {
            sigaddset(&mut set, sig);
            signalfd_update(&self.sfd.as_socket().0, &set)?;
            cell.set(set);
            Ok(true)
        }
    }

    pub fn del(&self, sig: Signal) -> Result<bool> {
        let cell = self.sfd.as_data();
        let mut set = cell.get();
        if sigismember(&set, sig) {
            sigdelset(&mut set, sig);
            signalfd_update(&self.sfd.as_socket().0, &set)?;
            cell.set(set);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn clear(&self) -> Result<()> {
        let cell = self.sfd.as_data();
        let set = sigemptyset();
        signalfd_update(&self.sfd.as_socket().0, &set)?;
        cell.set(set);
        Ok(())
    }

    pub const fn set_timeout(&mut self, timeout: Duration) {
        self.t = Timeout::from_duration(timeout)
    }

    pub fn nb_wait(&self) -> Result<Signal> {
        nb_wait(self.sfd.as_socket())
    }

    pub fn wait(&self) -> Result<Signal> {
        wait(self.sfd.as_socket(), self.as_ctx(), self.t)
    }

    pub async fn async_wait(&self) -> Result<Signal> {
        let mut ssi = MaybeUninit::<libc::signalfd_siginfo>::uninit();
        unsafe {
            let buf = slice::from_raw_parts_mut(
                ssi.as_mut_ptr() as *mut u8,
                size_of::<libc::signalfd_siginfo>(),
            );
            self.sfd.async_read(buf, self.t).await?;
            let ssi = ssi.assume_init();
            Ok(Signal::from_signalfd_siginfo(&ssi))
        }
    }
}

impl From<SignalSet> for AsyncSignalSet {
    fn from(sfd: SignalSet) -> Self {
        Self {
            sfd: AsyncSocket::new(sfd.ctx, sfd.sfd, sfd.set),
            t: sfd.t,
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
