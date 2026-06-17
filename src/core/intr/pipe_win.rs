use crate::core::clock::Deadline;
use crate::error::OsError;
use crate::primitive::Handle;
use std::cell::Cell;
use std::time::Duration;

pub(in super::super) struct Pipe {
    rfd: Handle,
    wfd: Handle,
    deadline: Cell<Deadline>,
}

impl Pipe {
    pub fn new() -> Result<Self, OsError> {
        let (rfd, wfd) = Handle::pipe()?;
        Ok(Self {
            rfd: rfd,
            wfd: wfd,
            deadline: Cell::new(Deadline::now()),
        })
    }

    pub fn as_handle(&self) -> &Handle {
        &self.rfd
    }

    pub fn timeout(&self) -> Duration {
        self.deadline.get().elapsed()
    }

    pub fn wake_up_now(&self) {
        self.wfd.write(&[1u8]).unwrap();
    }

    pub fn wake_up_alarm(&self, timer: Deadline) {
        self.deadline.set(timer);
    }

    pub fn update_event(&self) {
        self.rfd.read(&mut [0u8; 1]).unwrap();
    }
}

unsafe impl Send for Pipe {}
unsafe impl Sync for Pipe {}
