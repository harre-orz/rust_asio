use super::Deadline;
use crate::error::Result;
use crate::socket::Handle;
use std::cell::Cell;
use std::time::Duration;

pub(super) struct Pipe {
    rfd: Handle,
    wfd: Handle,
    timer: Cell<Deadline>,
}

impl Pipe {
    pub(super) fn new() -> Result<Self> {
        let (rfd, wfd) = Handle::pipe()?;
        Ok(Pipe {
            rfd: rfd,
            wfd: wfd,
            timer: Cell::new(Deadline::now()),
        })
    }

    pub(super) const fn as_handle(&self) -> &Handle {
        &self.rfd
    }

    pub(super) fn timeout(&self) -> Duration {
        self.timer.get().elapsed()
    }

    pub(super) fn wake_up_now(&self) {
        self.wfd.write(&[1u8]).unwrap();
    }

    pub(super) fn wake_up_alarm(&self, timer: Deadline) {
        self.timer.set(timer);
    }

    pub(super) fn update_event(&self) {
        self.rfd.read(&mut [0u8; 1]).unwrap();
    }
}

unsafe impl Send for Pipe {}
unsafe impl Sync for Pipe {}
