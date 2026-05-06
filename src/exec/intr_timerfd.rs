use super::Event;
use crate::error::OsError;
use crate::socket::Fd;
use std::ptr;
use std::result;
use std::sync::{Arc, Mutex};
use std::time::Instant;

type Result<T> = result::Result<T, OsError>;

fn timerfd_create() -> Result<Fd> {
    unsafe {
        match libc::timerfd_create(
            libc::CLOCK_MONOTONIC,
            libc::TFD_NONBLOCK | libc::TFD_CLOEXEC,
        ) {
            -1 => Err(OsError::last()),
            fd => Ok(Fd::new_unchecked(fd)),
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

fn now_to_timespec() -> libc::timespec {
    libc::timespec {
        tv_nsec: 0,
        tv_sec: 0,
    }
}

fn instant_to_timespec(cto: Instant) -> libc::timespec {
    let cto = cto.duration_since(Instant::now());
    libc::timespec {
        tv_nsec: cto.subsec_nanos() as i64,
        tv_sec: cto.as_secs() as i64,
    }
}

pub struct TimerFd {
    tfd: Fd,
    pub(crate) event: Arc<Mutex<Event>>,
}

impl TimerFd {
    pub fn new(event: Arc<Mutex<Event>>) -> Result<Self> {
        let tfd = timerfd_create()?;
        Ok(Self {
            tfd: tfd,
            event: event,
        })
    }

    pub fn as_raw_fd(&self) -> &Fd {
        &self.tfd
    }

    pub fn as_timeout_epoll(&self) -> i32 {
        -1
    }

    pub fn wake_up_now(&self) {
        timerfd_settime(&self.tfd, now_to_timespec())
    }

    pub fn wake_up_alarm(&self, cto: Instant) {
        timerfd_settime(&self.tfd, instant_to_timespec(cto))
    }

    pub fn read(&self) {
        let _ = self.tfd.read(&mut [0u8; 8]);
    }
}
