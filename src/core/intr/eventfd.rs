use crate::error::{OsError, Result};
use crate::primitive::{Deadline, Fd};
use std::cell::Cell;
use std::os::fd::RawFd;

fn eventfd() -> Result<Fd> {
    unsafe {
        match libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) {
            -1 => Err(OsError::last()),
            fd => Ok(Fd::from_raw_fd(fd)),
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
        Ok(
            Self {
                efd: efd,
                timer: Cell::new(Deadline::now()),
            }
        )
    }

    pub fn as_fd(&self) -> &Fd {
        &self.efd
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
