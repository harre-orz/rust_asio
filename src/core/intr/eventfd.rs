use super::Deadline;
use super::Fd;
use crate::error::OsError;
use crate::error::Result;
use std::cell::Cell;

fn eventfd() -> Result<Fd> {
    unsafe {
        match libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) {
            -1 => Err(OsError::last()),
            fd => Ok(Fd::new_unchecked(fd)),
        }
    }
}

pub(in super::super) struct EventFd {
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

    pub const fn as_fd(&self) -> &Fd {
        &self.efd
    }

    pub const unsafe fn as_native_handle(&self) -> libc::c_int {
        unsafe { self.efd.as_raw_fd() }
    }

    pub fn timeout_epoll(&self) -> i32 {
        self.timer.get().elapsed().as_millis() as i32
    }

    pub fn wake_up_now(&self) {
        let _ = self.efd.write(&[0, 0, 0, 0, 0, 0, 0, 1_u8]);
    }

    pub fn wake_up_alarm(&self, timer: Deadline) {
        self.timer.set(timer);
    }

    pub fn update_event(&self) -> bool {
        let _ = self.efd.read(&mut [0u8; 8]);
        true
    }
}

unsafe impl Send for EventFd {}
unsafe impl Sync for EventFd {}