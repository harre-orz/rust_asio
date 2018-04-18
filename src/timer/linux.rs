use super::TimerImpl;
use ffi::{AsRawFd, SystemError};
use core::{Handle, Reactor};

use libc::{timerfd_create, timerfd_settime, timespec, itimerspec, TFD_TIMER_ABSTIME,
           CLOCK_MONOTONIC, TFD_NONBLOCK, TFD_CLOEXEC};

pub struct TimerFd {
    tfd: Handle,
}

impl TimerFd {
    pub fn new() -> Result<Self, SystemError> {
        match unsafe { timerfd_create(CLOCK_MONOTONIC, TFD_NONBLOCK | TFD_CLOEXEC) } {
            -1 => Err(SystemError::last_error()),
            fd => Ok(TimerFd { tfd: Handle::intr(fd) }),
        }
    }

    pub fn startup(&self, reactor: &Reactor) {
        reactor.register_intr(&self.tfd)
    }

    pub fn cleanup(&self, reactor: &Reactor) {
        reactor.deregister_intr(&self.tfd)
    }

    pub fn wait_duration(&self, max: usize) -> usize {
        max
    }

    pub fn reset_timeout(&self, timer: &TimerImpl) {
        use std::ptr;

        let iti = itimerspec {
            it_interval: timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            it_value: timer.expiry.abs_time(),
        };
        unsafe {
            timerfd_settime(
                self.tfd.as_raw_fd(),
                TFD_TIMER_ABSTIME,
                &iti,
                ptr::null_mut(),
            );
        }
    }
}
