use super::Deadline;
use super::Fd;
use crate::error::OsError;
use crate::error::Result;
use std::cell::Cell;

pub(super) fn eventfd() -> Result<Fd> {
    unsafe {
        match libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) {
            -1 => Err(OsError::last()),
            fd => Ok(Fd::new_unchecked(fd)),
        }
    }
}

pub struct EventFd {
    efd: Fd,
    timer: Cell<Deadline>,
}

impl EventFd {
    pub fn new() -> Result<Self> {
        let efd = eventfd()?;
        Ok(Self {
            efd: efd,
            timer: Cell::new(Deadline::now()),
        })
    }

    pub(super) const fn as_fd(&self) -> &Fd {
        &self.efd
    }

    pub(super) const unsafe fn as_native_handle(&self) -> libc::c_int {
        unsafe { self.efd.as_raw_fd() }
    }

    pub(super) fn timeout_epoll(&self) -> i32 {
        self.timer.get().elapsed().as_millis() as i32
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
