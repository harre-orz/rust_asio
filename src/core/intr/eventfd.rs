use crate::error::{OsError, Result};
use crate::primitive::{Deadline, Fd};
use std::cell::Cell;

fn eventfd() -> Result<Fd> {
    unsafe {
        match libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) {
            -1 => Err(OsError::last()),
            fd => Ok(Fd::from_raw_fd(fd)),
        }
    }
}

pub(in super::super) struct EventFd {
    timer: Cell<Deadline>,
}

impl EventFd {
    pub fn new() -> Result<(Self, Fd)> {
        let efd = eventfd()?;
        Ok((
            Self {
                timer: Cell::new(Deadline::now()),
            },
            efd,
        ))
    }

    pub fn timeout_epoll(&self) -> i32 {
        self.timer.get().elapsed().as_millis() as i32
    }

    pub fn wake_up_now(&self, efd: &Fd) {
        let _ = efd.write(&[0, 0, 0, 0, 0, 0, 0, 1_u8]);
    }

    pub fn wake_up_alarm(&self, efd: &Fd, timer: Deadline) {
        self.timer.set(timer);
    }

    pub fn update_event(&self, efd: &Fd) -> bool {
        let _ = efd.read(&mut [0u8; 8]);
        true
    }
}

unsafe impl Send for EventFd {}
unsafe impl Sync for EventFd {}
