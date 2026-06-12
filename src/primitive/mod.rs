use std::time::Duration;

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

    pub(crate) const fn millis(&self) -> i32 {
        self.0
    }
}

#[cfg(not(all(target_os = "linux", feature = "timerfd")))]
#[derive(Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Debug)]
pub(crate) struct Deadline(Instant);

#[cfg(not(all(target_os = "linux", feature = "timerfd")))]
impl Deadline {
    pub(super) fn now() -> Self {
        Deadline(Instant::now())
    }

    pub(crate) fn new(timeout: Timeout) -> Self {
        Self(Instant::now() + timeout.into_duration())
    }
}

#[cfg(all(target_os = "linux", feature = "timerfd"))]
mod timerfd;
pub(crate) use self::timerfd::Deadline;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub(crate) use self::unix::Fd;
#[cfg(unix)]
pub use self::unix::{Signal, Socket};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::Socket;
#[cfg(windows)]
pub(crate) use self::windows::{AsRawHandle, Handle};
