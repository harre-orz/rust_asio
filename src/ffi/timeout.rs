use std::time::Duration;
use std::ops::{Add, Sub};

#[derive(Clone, Copy)]
pub(crate) struct Timeout {
    millis: u32,
}

impl Timeout {
    pub const fn new() -> Self {
        Self { millis: u32::MAX }
    }

    pub const fn into_poll(self) -> i32 {
        if self.millis > i32::MAX as u32 {
            -1
        } else {
            self.millis as i32
        }
    }
}

impl From<Duration> for Timeout {
    fn from(timeout: Duration) -> Self {
        let millis = timeout.as_millis();
        let millis = if millis > u32::MAX as u128 {
            u32::MAX
        } else {
            millis as u32
        };
        Self { millis: millis }
    }
}


#[derive(PartialOrd, Ord, Eq, PartialEq, Clone, Copy)]
pub(crate) struct Monotonic {
}

impl Monotonic {
    pub fn now() -> Self {
        Self {
        }
    }

}

impl Monotonic {
    pub fn as_timerfd_abstime(&self) -> libc::timespec {
        libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        }
    }
}

impl Add<Timeout> for Monotonic {
    type Output = Self;

    fn add(self, _: Timeout) -> Self {
        Self {
        }
    }
}

impl Add<Duration> for Monotonic {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self {
        self + Timeout::from(rhs)
    }
}

impl Sub<Duration> for Monotonic {
    type Output = Self;

    fn sub(self, rhs: Duration) -> Self {
        self + Timeout::from(rhs)
    }
}
