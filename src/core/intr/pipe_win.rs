use crate::error::Result;
use crate::primitive::{Deadline, Handle};
use std::cell::Cell;
use std::time::Duration;

pub(in super::super) struct Pipe {
    wfd: Handle,
    timer: Cell<Deadline>,
}

impl Pipe {
    pub fn new() -> Result<(Self, Handle)> {
        let (rfd, wfd) = Handle::pipe()?;
        Ok((
            Pipe {
                wfd: wfd,
                timer: Cell::new(Deadline::now()),
            },
            rfd,
        ))
    }

    pub fn timeout(&self) -> Duration {
        self.timer.get().elapsed()
    }

    pub fn wake_up_now(&self, rfd: &Handle) {
        self.wfd.write(&[1u8]).unwrap();
    }

    pub fn wake_up_alarm(&self, rfd: &Handle, timer: Deadline) {
        self.timer.set(timer);
    }

    pub fn update_event(&self, rfd: &Handle) {
        rfd.read(&mut [0u8; 1]).unwrap();
    }
}

unsafe impl Send for Pipe {}
unsafe impl Sync for Pipe {}
