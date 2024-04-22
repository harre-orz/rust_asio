use crate::OsError;
use crate::signal_set::Signal;
use std::mem;
use std::mem::MaybeUninit;
use std::os::fd::{FromRawFd, AsRawFd, OwnedFd};

type Result<T> = std::result::Result<T, OsError>;

pub fn sigemptyset() -> libc::sigset_t {
    let mut mask = MaybeUninit::<libc::sigset_t>::uninit();
    unsafe {
        libc::sigemptyset(mask.as_mut_ptr());
        mask.assume_init()
    }
}

pub fn sigfillset() -> libc::sigset_t {
    let mut mask = MaybeUninit::<libc::sigset_t>::uninit();
    unsafe {
        libc::sigfillset(mask.as_mut_ptr());
        mask.assume_init()
    }
}

pub fn sigaddset(mask: &mut libc::sigset_t, sig: Signal) -> Result<()> {
    unsafe {
        match libc::sigaddset(mask, sig.into()) {
            -1 => Err(OsError::last()),
            0 => Ok(()),
            _ => unreachable!(),
        }
    }
}

pub fn sigprocmask(how: i32, set: &libc::sigset_t) -> Result<libc::sigset_t> {
    let mut oset = MaybeUninit::<libc::sigset_t>::uninit();
    unsafe {
        match libc::sigprocmask(how, set, oset.as_mut_ptr()) {
            -1 => Err(OsError::last()),
            0 => Ok(oset.assume_init()),
            _ => unreachable!(),
        }
    }
}

pub fn signalfd(mask: &libc::sigset_t) -> Result<OwnedFd>
{
    unsafe {
        match libc::signalfd(-1, mask, libc::SFD_NONBLOCK | libc::SFD_CLOEXEC) {
            -1 => Err(OsError::last()),
            sfd => Ok(OwnedFd::from_raw_fd(sfd)),
        }
    }
}

pub fn signal_read(sfd: &OwnedFd) -> Result<Signal>
{
    let mut ssi = MaybeUninit::<libc::signalfd_siginfo>::uninit();
    const LEN: isize = mem::size_of::<libc::signalfd_siginfo>() as isize;
    unsafe {
        match libc::read(
            sfd.as_raw_fd(),
            ssi.as_mut_ptr().cast(),
            mem::size_of_val(&ssi),
        ) {
            -1 => Err(OsError::last()),
            0 => Err(OsError::CONNECTION_ABORTED),
            LEN => Ok(Signal::from_raw(ssi.assume_init().ssi_signo)),
            _ => unreachable!(),
        }
    }
}
