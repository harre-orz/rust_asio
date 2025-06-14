use super::Event;
use crate::error::OsError;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::sync::{Arc, Mutex};

#[cfg(target_os = "linux")]
mod ffi {
    use super::*;

    pub fn eventfd() -> Result<OwnedFd, OsError> {
        match unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) } {
            -1 => Err(unsafe { OsError::last() }),
            fd => Ok(unsafe { OwnedFd::from_raw_fd(fd) }),
        }
    }
}

#[cfg(target_os = "macos")]
mod ffi {}

pub struct EventFd {
    efd: OwnedFd,
    pub event: Arc<Mutex<Event>>,
}

impl EventFd {
    #[cfg(target_os = "linux")]
    pub fn new(event: Arc<Mutex<Event>>) -> Result<Self, OsError> {
        let efd = ffi::eventfd()?;
        Ok(Self {
            efd: efd,
            event: event,
        })
    }

    pub fn intr(&self) {
        let buf = [0, 0, 0, 0, 0, 0, 0, 1_u8];
        match unsafe { libc::write(self.efd.as_raw_fd(), buf.as_ptr().cast(), buf.len()) } {
            8 => return,
            _ => panic!(),
        }
    }

    pub fn read(&self) {
        let mut buf = MaybeUninit::<[u8; 8]>::uninit();
        match unsafe { libc::read(self.efd.as_raw_fd(), buf.as_mut_ptr().cast(), 8) } {
            8 => return,
            _ => panic!(),
        }
    }
}
