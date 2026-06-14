use crate::error::{OsError, Result};
use std::mem::MaybeUninit;
use std::ptr;

pub use crate::primitive::Signal;

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

fn sigmask(how: i32, set: &libc::sigset_t) -> Result<()> {
    unsafe {
        match libc::pthread_sigmask(how, set, ptr::null_mut()) {
            -1 => Err(OsError::last()),
            _ => Ok(()),
        }
    }
}

fn sigismember(set: &libc::sigset_t, sig: Signal) -> bool {
    match unsafe { libc::sigismember(set, sig.number()) } {
        0 => false,
        1 => true,
        _ => panic!(),
    }
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use self::linux::{AsyncSignalSet, SignalSet};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::{AsyncSignalSet, SignalSet};
