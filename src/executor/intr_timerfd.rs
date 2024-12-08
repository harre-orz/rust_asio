use super::Event;
use crate::error::OsError;
use std::os::fd::{OwnedFd, FromRawFd, AsRawFd, RawFd};
use std::mem::MaybeUninit;
use std::sync::{Arc, Mutex};
use std::time::Instant;

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

    pub fn timerfd_settime(tfd: &OwnedFd, time: Instant) {
        // if let Some(it) = clo.into_itimerspec() {
        //     match unsafe {
	// 	libc::timerfd_settime(
        //             tfd.as_raw_fd(),
        //             0,
        //             &it,
        //             ptr::null_mut(),
	// 	)
	//     } {
	// 	0 => return,
	// 	_ => panic!(),
        //     }
	// }
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

    // pub fn reset(&self, clock: DeadlineClock) {
    // }

    pub fn as_timeout_epoll(&self) -> i32 {
	-1
    }

    pub fn intr(&self) {
	//ffi::timerfd_settime(&self.tfd, DeadlineClock::now())
    }
    
    pub fn read(&self) {
        let mut buf = MaybeUninit::<[u8; 8]>::uninit();
	match unsafe {
            libc::read(self.tfd.as_raw_fd(), buf.as_mut_ptr().cast(), 8)
	} {
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
