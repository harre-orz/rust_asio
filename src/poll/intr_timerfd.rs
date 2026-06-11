use super::Deadline;
use super::Fd;
use crate::error::OsError;
use crate::error::Result;
use std::ptr;

fn timerfd_create() -> Result<Fd> {
    unsafe {
        match libc::timerfd_create(
            libc::CLOCK_MONOTONIC, libc::TFD_NONBLOCK | libc::TFD_CLOEXEC,
        ) {
            -1 => Err(OsError::last()),
            fd => Ok(Fd::from_raw_fd(fd)),
        }
    }
}

fn timerfd_settime(tfd: &Fd, tv: libc::timespec) {
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

pub(crate) struct TimerFd {
    tfd: Fd,
}

impl TimerFd {
    pub(crate) fn new() -> Result<Self> {
        let tfd = timerfd_create()?;
        Ok(Self { tfd: tfd })
    }

    pub(crate) const fn as_fd(&self) -> &Fd {
        &self.tfd
    }

    pub(crate) fn timeout_epoll(&self) -> i32 {
        -1
    }

    pub(crate) fn wake_up_now(&self) {
        let tv = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        timerfd_settime(&self.tfd, tv)
    }

    pub(crate) fn wake_up_alarm(&self, timer: Deadline) {
        timerfd_settime(&self.tfd, timer.as_absolute_timespec())
    }

    pub(crate) fn update_event(&self) -> bool {
        let _ = self.tfd.read(&mut [0u8; 8]);
        true
    }
}
