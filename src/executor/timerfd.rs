//

use socket_base::NativeHandle;
use error::ErrorCode;
use libc;

use std::cell::Cell;
use std::time::Instant;

pub struct Intr {
    tfd: NativeHandle,
    expire: Cell<Instant>,
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
            tfd: tfd,
            expire: Cell::new(Instant::now()),
        })
    }

    pub fn native_handle(&self) -> NativeHandle {
        self.tfd
    }

    pub fn interrupt(&self) {
    }

    pub fn reset_timeout(&self, expire: Instant) {
        use std::ptr;

        if expire >= self.expire.get() {
            return
        }

        let now = Instant::now();
        if expire <= now {
            return
        }
        self.expire.set(expire);
        let dur = expire - Instant::now();
        let iti = libc::itimerspec {
            it_interval: libc::timespec {
                tv_sec: dur.as_secs() as i64,
                tv_nsec: dur.subsec_nanos() as i64,
            },
            it_value: libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
        };
        let _ = unsafe {
            libc::timerfd_settime(
                self.tfd,
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
