use crate::OsError;
use crate::Signal;
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
        let err = libc::sigaddset(mask, sig.into());
        if err < 0 {
            return Err(OsError::last());
        }
        Ok(())
    }
}

pub fn sigprocmask(how: i32, set: &libc::sigset_t) -> Result<libc::sigset_t> {
    let mut oset = MaybeUninit::<libc::sigset_t>::uninit();
    unsafe {
        let err = libc::sigprocmask(how, set, oset.as_mut_ptr());
        if err < 0 {
            return Err(OsError::last());
        }
        Ok(oset.assume_init())
    }
}

pub fn signalfd(mask: &libc::sigset_t) -> Result<OwnedFd>
{
    unsafe {
        let fd = libc::signalfd(-1, mask, libc::SFD_NONBLOCK | libc::SFD_CLOEXEC);
        if fd < 0 {
            return Err(OsError::last());
        }
        Ok(OwnedFd::from_raw_fd(fd))
    }
}

pub fn signalfd_read(sig: &OwnedFd) -> Result<Signal>
{
    let mut ssi = MaybeUninit::<libc::signalfd_siginfo>::uninit();
    unsafe {
        let len = libc::read(
            sig.as_raw_fd(),
            ssi.as_mut_ptr().cast(),
            mem::size_of_val(&ssi),
        );
        if len < 0 {
            return Err(OsError::last());
        }
        let ssi = ssi.assume_init();
        Ok(Signal(ssi.ssi_signo as i32))
    }
}
