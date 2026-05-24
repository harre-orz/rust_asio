use super::Deadline;
use crate::error::Result;
use crate::socket::Fd;
use std::cell::Cell;

#[cfg(target_os = "linux")]
mod ffi {
    use crate::error::{OsError, Result};
    use crate::socket::Fd;

    pub(super) fn eventfd() -> Result<Fd> {
        unsafe {
            match libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) {
                -1 => Err(OsError::last()),
                fd => Ok(Fd::new_unchecked(fd)),
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod ffi {
    use crate::error::Result;
    use crate::socket::Fd;

    pub(super) fn eventfd() -> Result<Fd> {
        unimplemented!("")
    }
}

pub struct EventFd {
    efd: Fd,
    timer: Cell<Deadline>,
}

impl EventFd {
    pub fn new() -> Result<Self> {
        let efd = ffi::eventfd()?;
        Ok(Self {
            efd: efd,
            timer: Cell::new(Deadline::now()),
        })
    }

    pub(super) const fn as_fd(&self) -> &Fd {
        &self.efd
    }

    #[cfg(unix)]
    pub(super) const unsafe fn as_native_handle(&self) -> libc::c_int {
        unsafe { self.efd.as_raw_fd() }
    }

    #[cfg(feature = "poll_epoll")]
    pub(super) fn timeout_epoll(&self) -> i32 {
        self.timer.get().as_relative_millis()
    }

    pub(super) fn wake_up_now(&self) {
        let _ = self.efd.write(&[0, 0, 0, 0, 0, 0, 0, 1_u8]);
    }

    pub(super) fn wake_up_alarm(&self, timer: Deadline) {
        self.timer.set(timer);
    }

    pub(super) fn update_event(&self) -> bool {
        let _ = self.efd.read(&mut [0u8; 8]);
        true
    }
}

unsafe impl Send for EventFd {}
unsafe impl Sync for EventFd {}
