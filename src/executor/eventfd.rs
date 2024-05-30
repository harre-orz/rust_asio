use super::Event;
use crate::error::OsError;
use crate::ffi::Monotonic;
use std::cell::Cell;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, OwnedFd, RawFd};

mod ffi {
    use crate::executor::eventfd::OsError;
    use libc;
    use std::os::fd::{FromRawFd, OwnedFd};

    pub fn eventfd() -> Result<OwnedFd, OsError> {
        match unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) } {
            -1 => Err(unsafe { OsError::last() }),
            fd => Ok(unsafe { OwnedFd::from_raw_fd(fd) }),
        }
    }
}

pub struct EventFdIntr {
    efd: OwnedFd,
    event: Event,
    timeout: Cell<i32>,
}

impl EventFdIntr {
    pub fn new() -> Result<Self, OsError> {
        let efd = ffi::eventfd()?;
        Ok(Self {
            efd: efd,
            event: Event::intr(),
            timeout: Cell::new(0),
        })
    }

    pub const fn as_event(&self) -> &Event {
        &self.event
    }

    pub fn timeout_for_epoll(&self) -> i32 {
        self.timeout.get()
    }

    pub fn reset(&self, deadline: Monotonic) {
        self.timeout.set(deadline.timeout_at_now().as_millis_i32());
        self.intr()
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

impl AsRawFd for EventFdIntr {
    fn as_raw_fd(&self) -> RawFd {
        self.efd.as_raw_fd()
    }
}
