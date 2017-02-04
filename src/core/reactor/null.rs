use super::{Dispatcher, IntrFdImpl, AsyncFdImpl};
use core::{Scheduler, Interrupter, WorkerQueue};

use std::io;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

type Dispatch = fn();
pub type IntrFd = IntrFdImpl<Dispatch>;
pub type AsyncFd = AsyncFdImpl<Dispatch>;
pub type Reactor = NullReactor;

pub struct NullReactor;

impl NullReactor {
    pub fn new() -> io::Result<Self> {
        Ok(NullReactor)
    }

    pub fn run(&self, schd: &Scheduler, _block: bool, wq: &mut WorkerQueue) -> bool {
        schd.get_ready_timers(wq);
        false
    }

    pub fn cancel_all_fds(&self, _wq: &mut WorkerQueue) {
    }
}

impl Dispatcher for Interrupter {
    type Dispatch = Dispatch;

    fn dispatcher() -> Self::Dispatch {
        Self::dispatch
    }
}

impl Interrupter {
    fn dispatch() {
    }
}
