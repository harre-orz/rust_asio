use crate::core::{Fd, Timeout};
use crate::error::{OsError, Result};
use crate::socket::{AsyncSocket, Socket};
use crate::{IoContext, core, socket};
use std::mem::MaybeUninit;
use std::time::Duration;
use std::{ptr, slice};

pub use crate::core::Signal;

fn sigemptyset() -> libc::sigset_t {
    let mut mask = MaybeUninit::<libc::sigset_t>::uninit();
    unsafe {
        libc::sigemptyset(mask.as_mut_ptr());
        mask.assume_init()
    }
}

fn sigaddset(mask: &mut libc::sigset_t, sig: Signal) {
    unsafe {
        libc::sigaddset(mask, sig.number());
    }
}

fn sigdelset(mask: &mut libc::sigset_t, sig: Signal) {
    unsafe {
        libc::sigdelset(mask, sig.number());
    }
}

fn sigmaskget() -> Result<libc::sigset_t> {
    let mut oldset = MaybeUninit::<libc::sigset_t>::uninit();
    unsafe {
        match libc::pthread_sigmask(libc::SIG_BLOCK, ptr::null(), oldset.as_mut_ptr()) {
            -1 => Err(OsError::last()),
            _ => Ok(oldset.assume_init()),
        }
    }
}

fn sigmaskset(how: i32, set: &libc::sigset_t) -> Result<()> {
    unsafe {
        match libc::pthread_sigmask(how, set, ptr::null_mut()) {
            -1 => Err(OsError::last()),
            _ => Ok(()),
        }
    }
}

fn signalfd(mask: &libc::sigset_t) -> Result<Fd> {
    unsafe {
        match libc::signalfd(-1, mask, libc::SFD_NONBLOCK | libc::SFD_CLOEXEC) {
            -1 => Err(OsError::last()),
            sfd => Ok(Fd::from_raw_fd(sfd)),
        }
    }
}

fn signalfd_(fd: &Fd, mask: &libc::sigset_t) -> Result<()> {
    unsafe {
        match libc::signalfd(fd.as_raw_fd(), mask, 0) {
            -1 => Err(OsError::last()),
            _ => Ok(()),
        }
    }
}

fn nb_wait(soc: &Socket) -> Result<Signal> {
    let mut ssi = MaybeUninit::<libc::signalfd_siginfo>::uninit();
    unsafe {
        let buf = slice::from_raw_parts_mut(
            ssi.as_mut_ptr() as *mut u8,
            size_of::<libc::signalfd_siginfo>(),
        );
        soc.read(buf)?;
        let ssi = ssi.assume_init();
        Ok(Signal::from_signalfd_siginfo(&ssi))
    }
}

async fn async_wait(soc: &AsyncSocket, timeout: Timeout) -> Result<Signal> {
    let mut ssi = MaybeUninit::<libc::signalfd_siginfo>::uninit();
    unsafe {
        let buf = slice::from_raw_parts_mut(
            ssi.as_mut_ptr() as *mut u8,
            size_of::<libc::signalfd_siginfo>(),
        );
        soc.read_some(buf, timeout).await?;
        let ssi = ssi.assume_init();
        Ok(Signal::from_signalfd_siginfo(&ssi))
    }
}

fn wait(soc: &Socket, timeout: Timeout) -> Result<Signal> {
    let mut ssi = MaybeUninit::<libc::signalfd_siginfo>::uninit();
    unsafe {
        let buf = slice::from_raw_parts_mut(
            ssi.as_mut_ptr() as *mut u8,
            size_of::<libc::signalfd_siginfo>(),
        );
        socket::read_some(soc, buf, timeout)?;
        let ssi = ssi.assume_init();
        Ok(Signal::from_signalfd_siginfo(&ssi))
    }
}

struct SignalSetGuard(libc::sigset_t);

impl Drop for SignalSetGuard {
    fn drop(&mut self) {
        let _ = sigmaskset(libc::SIG_SETMASK, &self.0);
    }
}

pub struct AsyncSignalSet {
    sfd: AsyncSocket,
    _set: SignalSetGuard,
    timeout: Timeout,
}

impl AsyncSignalSet {
    pub fn as_ctx(&self) -> &IoContext {
        self.sfd.as_ctx()
    }

    pub fn add(&self, sig: Signal) -> Result<()> {
        let mut mask = sigmaskget()?;
        sigaddset(&mut mask, sig);
        sigmaskset(libc::SIG_SETMASK, &mask)?;
        Ok(signalfd_(self.sfd.as_socket().as_fd(), &mask)?)
    }

    pub fn del(&self, sig: Signal) -> Result<()> {
        let mut mask = sigmaskget()?;
        sigdelset(&mut mask, sig);
        sigmaskset(libc::SIG_SETMASK, &mask)?;
        Ok(signalfd_(self.sfd.as_socket().as_fd(), &mask)?)
    }

    pub fn clear(&self) -> Result<()> {
        let mask = sigemptyset();
        sigmaskset(libc::SIG_SETMASK, &mask)?;
        Ok(signalfd_(self.sfd.as_socket().as_fd(), &mask)?)
    }

    pub const fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = Timeout::from_duration(timeout)
    }

    pub fn nb_wait(&self) -> Result<Signal> {
        nb_wait(self.sfd.as_socket())
    }

    pub async fn async_wait(&self) -> Result<Signal> {
        async_wait(&self.sfd, self.timeout).await
    }
}

pub struct SignalSet {
    sfd: Socket,
    _set: SignalSetGuard,
    timeout: Timeout,
}

impl SignalSet {
    pub fn new(ctx: &IoContext) -> Result<SignalSet> {
        let mask = sigmaskget()?;
        let sfd = unsafe { Socket::from_raw_fd(ctx.clone(), signalfd(&mask)?) };
        Ok(SignalSet {
            sfd: sfd,
            _set: SignalSetGuard(mask),
            timeout: Timeout::infinite(),
        })
    }

    pub fn with_signals<T>(ctx: &IoContext, signals: T) -> Result<SignalSet>
    where
        T: AsRef<[Signal]>,
    {
        let mut mask = sigmaskget()?;
        for sig in signals.as_ref() {
            sigaddset(&mut mask, *sig);
        }
        sigmaskset(libc::SIG_SETMASK, &mask)?;
        let sfd = unsafe { Socket::from_raw_fd(ctx.clone(), signalfd(&mask)?) };
        Ok(SignalSet {
            sfd: sfd,
            _set: SignalSetGuard(mask),
            timeout: Timeout::infinite(),
        })
    }

    pub fn as_ctx(&self) -> &IoContext {
        self.sfd.as_ctx()
    }

    pub fn add(&self, sig: Signal) -> Result<()> {
        let mut mask = sigmaskget()?;
        sigaddset(&mut mask, sig);
        sigmaskset(libc::SIG_SETMASK, &mask)?;
        Ok(signalfd_(self.sfd.as_fd(), &mask)?)
    }

    pub fn del(&self, sig: Signal) -> Result<()> {
        let mut mask = sigmaskget()?;
        sigdelset(&mut mask, sig);
        sigmaskset(libc::SIG_SETMASK, &mask)?;
        Ok(signalfd_(self.sfd.as_fd(), &mask)?)
    }

    pub fn clear(&self) -> Result<()> {
        let mask = sigemptyset();
        sigmaskset(libc::SIG_SETMASK, &mask)?;
        Ok(signalfd_(self.sfd.as_fd(), &mask)?)
    }

    pub const fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = Timeout::from_duration(timeout)
    }

    pub fn nb_wait(&self) -> Result<Signal> {
        nb_wait(&self.sfd)
    }

    pub fn wait(&self) -> Result<Signal> {
        wait(&self.sfd, self.timeout)
    }
}

impl From<SignalSet> for AsyncSignalSet {
    fn from(sfd: SignalSet) -> Self {
        Self {
            sfd: AsyncSocket::new(sfd.sfd),
            _set: sfd._set,
            timeout: sfd.timeout,
        }
    }
}
