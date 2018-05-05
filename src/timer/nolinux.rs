use super::TimerImpl;
use ffi::SystemError;
use reactor::Reactor;

use std::cmp;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct TimerCtl {
    timeout_nsec: AtomicUsize,
}

impl TimerCtl {
    pub fn new() -> Result<Self, SystemError> {
        Ok(TimerCtl { timeout_nsec: AtomicUsize::new(0) })
    }

    pub fn startup(&self, _: &Reactor) {}

    pub fn cleanup(&self, _: &Reactor) {}

    pub fn wait_duration(&self, max: usize) -> usize {
        cmp::min(self.timeout_nsec.load(Ordering::Relaxed), max)
    }

    pub fn reset_timeout(&self, timer: &TimerImpl) {
        self.timeout_nsec.store(
            timer.expiry.left(),
            Ordering::SeqCst,
        );
        timer.ctx.as_reactor().interrupt();
    }
}
