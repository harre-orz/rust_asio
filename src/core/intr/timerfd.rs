use super::Deadline;
use crate::error::OsError;
use crate::error::Result;
use crate::primitive::Fd;
use std::ptr;

fn timerfd_create() -> Result<Fd> {
    unsafe {
        match libc::timerfd_create(
            libc::CLOCK_MONOTONIC,
            libc::TFD_NONBLOCK | libc::TFD_CLOEXEC,
        ) {
            -1 => Err(OsError::last()),
            fd => Ok(Fd::from_raw_fd(fd)),
        }
    }
}

fn timerfd_settime(tfd: &Fd, timer: Deadline) {
    let it = libc::itimerspec {
        it_interval: libc::timespec {
            tv_nsec: 0,
            tv_sec: 0,
        },
        it_value: timer.absolute(),
    };
    unsafe {
        match libc::timerfd_settime(tfd.as_raw_fd(), libc::TIMER_ABSTIME, &it, ptr::null_mut()) {
            0 => return,
            _ => panic!(),
        }
    }
}

pub(in super::super) struct TimerFd;

impl TimerFd {
    pub fn new() -> Result<(Self, Fd)> {
        let tfd = timerfd_create()?;
        Ok((TimerFd, tfd))
    }

    pub fn timeout_epoll(&self) -> i32 {
        -1
    }

    pub fn wake_up_now(&self, tfd: &Fd) {
        timerfd_settime(tfd, Deadline::now())
    }

    pub fn wake_up_alarm(&self, tfd: &Fd, timer: Deadline) {
        timerfd_settime(tfd, timer)
    }

    pub fn update_event(&self, tfd: &Fd) -> bool {
        let _ = tfd.read(&mut [0u8; 8]);
        true
    }
}
