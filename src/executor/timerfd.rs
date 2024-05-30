use super::Event;
use crate::error::OsError;
use crate::ffi::Monotonic;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, OwnedFd, RawFd};

mod ffi {
    use crate::error::OsError;
    use crate::ffi::Monotonic;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::ptr;

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

    pub fn timerfd_settime(tfd: &OwnedFd, time: Monotonic) {
        let it = time.into_itimerspec();
        match unsafe {
            libc::timerfd_settime(
                tfd.as_raw_fd(),
                libc::TFD_TIMER_ABSTIME,
                &it,
                ptr::null_mut(),
            )
        } {
            0 => {}
            _ => panic!(),
        }
    }
}

pub(super) struct TimerFdIntr {
    tfd: OwnedFd,
    event: Event,
}

impl TimerFdIntr {
    pub fn new() -> Result<Self, OsError> {
        let tfd = ffi::timerfd_create()?;
        ffi::timerfd_settime(&tfd, Monotonic::now());
        Ok(Self {
            tfd: tfd,
            event: Event::intr(),
        })
    }

    pub const fn as_event(&self) -> &Event {
        &self.event
    }

    pub const fn timeout_for_epoll(&self) -> i32 {
        -1
    }

    pub fn reset(&self, deadline: Monotonic) {
        ffi::timerfd_settime(&self.tfd, deadline)
    }

    pub fn intr(&self) {
        self.reset(Monotonic::now())
    }

    pub fn read(&self) {
        let mut buf = MaybeUninit::<[u8; 8]>::uninit();
        unsafe {
            match libc::read(self.tfd.as_raw_fd(), buf.as_mut_ptr().cast(), 8) {
                8 => return,
                _ => panic!(),
            }
        }
    }
}

impl AsRawFd for TimerFdIntr {
    fn as_raw_fd(&self) -> RawFd {
        self.tfd.as_raw_fd()
    }
}
