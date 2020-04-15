//

use super::{Reactor, SocketContext, callback_intr};
use error::ErrorCode;
use libc;

use std::time::Duration;

pub struct Intr {
    tfd: SocketContext,
}

impl Intr {
    pub fn new() -> Result<Self, ErrorCode> {
        let tfd = unsafe {
            libc::timerfd_create(
                libc::CLOCK_MONOTONIC,
                libc::TFD_NONBLOCK | libc::TFD_CLOEXEC,
            )
        };
        if tfd < 0 {
            return Err(ErrorCode::last_error())
        }
        Ok(Intr {
            tfd: SocketContext {
                handle: tfd,
                callback: callback_intr,
            },
        })
    }

    pub fn startup(&self, reactor: &Reactor) {
        reactor.register_interrupter(&self.tfd);
    }

    pub fn cleanup(&self, reactor: &Reactor) {
        reactor.deregister_interrupter(&self.tfd);
    }

    pub fn interrupt(&self) {
    }

    pub fn reset_timeout(&self, expire: Duration) {
        use std::ptr;

        let iti = libc::itimerspec {
            it_interval: libc::timespec {
                tv_sec: expire.as_secs() as i64,
                tv_nsec: expire.subsec_nanos() as i64,
            },
            it_value: libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
        };
        let _ = unsafe {
            libc::timerfd_settime(
                self.tfd.handle,
                0,
                &iti,
                ptr::null_mut(),
            )
        };
    }

    pub fn wait_duration(&self) -> i32 {
        1000
    }
}
