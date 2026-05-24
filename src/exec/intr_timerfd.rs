use super::Deadline;
use crate::error::Result;
use crate::socket::Fd;

mod ffi {
    use crate::error::{OsError, Result};
    use crate::socket::Fd;
    use std::ptr;

    pub(super) fn timerfd_create() -> Result<Fd> {
        unsafe {
            match libc::timerfd_create(
                libc::CLOCK_MONOTONIC,
                libc::TFD_NONBLOCK | libc::TFD_CLOEXEC, /* | libc::TFD_TIMER_ABSTIME */
            ) {
                -1 => Err(OsError::last()),
                fd => Ok(Fd::new_unchecked(fd)),
            }
        }
    }

    pub(super) fn timerfd_settime(tfd: &Fd, tv: libc::timespec) {
        let it = libc::itimerspec {
            it_interval: libc::timespec {
                tv_nsec: 0,
                tv_sec: 0,
            },
            it_value: tv,
        };
        unsafe {
            match libc::timerfd_settime(tfd.as_raw_fd(), 0, &it, ptr::null_mut()) {
                0 => return,
                _ => panic!(),
            }
        }
    }
}

pub(super) struct TimerFd {
    tfd: Fd,
}

impl TimerFd {
    pub(super) fn new() -> Result<Self> {
        let tfd = ffi::timerfd_create()?;
        Ok(Self { tfd: tfd })
    }

    pub(super) const fn as_fd(&self) -> &Fd {
        &self.tfd
    }

    #[cfg(unix)]
    pub(super) const unsafe fn as_native_handle(&self) -> libc::c_int {
        unsafe { self.tfd.as_raw_fd() }
    }

    #[cfg(feature = "poll_epoll")]
    pub(super) fn timeout_epoll(&self) -> i32 {
        -1
    }

    #[cfg(feature = "poll_kqueue")]
    pub(super) fn timeout_kqueue(&self) -> libc::timespec {
        libc::timespec {
            tv_sec: libc::time_t::MAX,
            tv_nsec: 0,
        }
    }

    #[cfg(feature = "poll_select")]
    pub(super) fn timeout_select(&self) -> libc::timeval {
        libc::timeval {
            tv_sec: libc::time_t::MAX,
            tv_usec: 0,
        }
    }

    pub(super) fn wake_up_now(&self) {
        let tv = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        ffi::timerfd_settime(&self.tfd, tv)
    }

    pub(super) fn wake_up_alarm(&self, timer: Deadline) {
        ffi::timerfd_settime(&self.tfd, timer.as_relative_timespec())
    }

    pub(super) fn update_event(&self) -> bool {
        let _ = self.tfd.read(&mut [0u8; 8]);
        true
    }
}
