use super::Event;
use crate::error::OsError;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::ptr;

mod ffi {
    use super::*;

    pub fn timerfd_create() -> Result<OwnedFd, OsError> {
        match unsafe {
            libc::timerfd_create(
                libc::CLOCK_MONOTONIC,
                libc::TFD_NONBLOCK | libc::TFD_CLOEXEC,
            )
        } {
            -1 => Err(unsafe { OsError::last() }),
            fd => Ok(unsafe { OwnedFd::from_raw_fd(fd) }),
        }
    }

    pub fn timerfd_settime(tfd: &OwnedFd, tv: libc::timespec) {
	let it = libc::itimerspec {
	    it_interval: libc::timespec {
		tv_nsec: 0,
		tv_sec: 0,
	    },
	    it_value: tv,
	};
        match unsafe {
            libc::timerfd_settime(
                tfd.as_raw_fd(),
                0,
                &it,
                ptr::null_mut(),
            )
        } {
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

fn instant_to_timespec(time: Instant) -> libc::timespec {
    let time = time.duration_since(Instant::now());
    libc::timespec {
	tv_nsec: time.subsec_nanos() as i64,
	tv_sec: time.as_secs() as i64,
    }
}


pub struct TimerFd {
    tfd: OwnedFd,
    pub event: Arc<Mutex<Event>>,
}

impl TimerFd {
    pub fn new(event: Arc<Mutex<Event>>) -> Result<Self, OsError> {
        let tfd = ffi::timerfd_create()?;
        Ok(Self {
            tfd: tfd,
            event: event,
        })
    }

    pub fn as_timeout_epoll(&self) -> i32 {
        -1
    }
    
    pub fn wake_up_now(&self) {
	ffi::timerfd_settime(&self.tfd, now_to_timespec())
    }

    pub fn wake_up_alarm(&self, time: Instant) {
	ffi::timerfd_settime(&self.tfd, instant_to_timespec(time))
    }

    pub fn read(&self) {
        let mut buf = MaybeUninit::<[u8; 8]>::uninit();
        match unsafe { libc::read(self.tfd.as_raw_fd(), buf.as_mut_ptr().cast(), 8) } {
            8 => return,
            _ => panic!(),
        }
    }
}

impl AsRawFd for TimerFd {
    fn as_raw_fd(&self) -> RawFd {
        self.tfd.as_raw_fd()
    }
}
