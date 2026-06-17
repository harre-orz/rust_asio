use crate::core::clock::Deadline;
use crate::error::OsError;
use crate::primitive::Fd;
use std::cell::Cell;

fn eventfd() -> Result<Fd, OsError> {
    unsafe {
        match libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) {
            -1 => Err(OsError::last()),
            fd => Ok(Fd::from_raw_fd(fd)),
        }
    }
}

pub(in super::super) struct EventFd {
    efd: Fd,
    deadline: Cell<Deadline>,
}

impl EventFd {
    pub fn new() -> Result<Self, OsError> {
        let efd = eventfd()?;
        Ok(Self {
            efd: efd,
            deadline: Cell::new(Deadline::now()),
        })
    }

    pub fn as_fd(&self) -> &Fd {
        &self.efd
    }

    pub fn timeout_epoll(&self) -> i32 {
        self.deadline.get().elapsed().as_millis() as i32
    }

    pub fn wake_up_now(&self) {
        let _ = self.efd.write(&[0, 0, 0, 0, 0, 0, 0, 1_u8]);
    }

    pub fn wake_up_alarm(&self, deadline: Deadline) {
        self.deadline.set(deadline);
    }

    pub fn update_event(&self) -> bool {
        let _ = self.efd.read(&mut [0u8; 8]);
        true
    }
}

unsafe impl Sync for EventFd {}
