use std::time::{Duration};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub(crate) struct Timeout(libc::c_int);

impl Timeout {
    pub const MAX: Self = Self(i32::MAX);

    pub const fn from_duration(timer: Duration) -> Self {
        let time = timer.as_millis();
        Self(if time > i32::MAX as u128 { i32::MAX } else { time as i32 })
    }

    pub fn as_millis(&self) -> libc::c_int {
        self.0
    }
}

// #[cfg(all(target_os = "linux", feature = "timerfd"))]
// mod clock;
// #[cfg(all(target_os = "linux", feature = "timerfd"))]
// pub(crate) use self::clock::Deadline;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub(crate) use self::unix::Fd;
#[cfg(unix)]
pub use self::unix::{Signal, Socket};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use self::windows::{AsRawHandle, Handle, Socket};
