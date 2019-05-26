//

use super::{Reactor, SocketContext};
use error::ErrorCode;
use libc;
use std::ptr;
use std::time::Instant;

pub struct Interrupter {
    efd: SocketContext,
    tfd: SocketContext,
}

impl Interrupter {
    pub fn new() -> Result<Self, ErrorCode> {
        let efd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
        let tfd = unsafe { libc::timerfd_create(libc::CLOCK_MONOTONIC, libc::TFD_NONBLOCK | libc::TFD_CLOEXEC) };
        if efd >= 0 {
            Ok(Interrupter {
                efd: SocketContext::interrupter(efd),
                tfd: SocketContext::interrupter(tfd),
            })
        } else {
            Err(ErrorCode::last_error())
        }
    }

    pub fn startup(&self, reactor: &Reactor) {
        reactor.register_interrupter(&self.efd);
        reactor.register_interrupter(&self.tfd);
    }

    pub fn cleanup(&self, reactor: &Reactor) {
        reactor.deregister_interrupter(&self.efd);
        reactor.deregister_interrupter(&self.tfd);
    }

    pub const fn wait_duration(&self, max: usize) -> usize {
        max
    }

    pub fn reset_timeout(&self, entry: Instant) {
        let iti = libc::itimerspec {
            it_interval: libc::timespec { tv_sec: 0, tv_nsec: 0 },
            it_value: libc::timespec { tv_sec: 0, tv_nsec: 0 },
        };
        let _ =
            unsafe { libc::timerfd_settime(self.tfd.native_handle(), libc::TFD_TIMER_ABSTIME, &iti, ptr::null_mut()) };
    }

    pub fn interrupt(&self) {
        let buf = [1, 0, 0, 0, 0, 0, 0, 0];
        let _ = unsafe { libc::write(self.efd.native_handle(), buf.as_ptr() as *const _, buf.len() as _) };
    }
}
