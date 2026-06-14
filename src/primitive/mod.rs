#[allow(dead_code)]
use std::time::{Duration, Instant};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub(crate) struct Timeout(libc::c_int);

impl Timeout {
    pub(crate) const INFINITE: Self = Self(-1);

    pub(crate) const fn from_duration(timeout: Duration) -> Self {
        let time = timeout.as_millis();
        if time > i32::MAX as u128 {
            Timeout::INFINITE
        } else {
            Timeout(time as i32)
        }
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
