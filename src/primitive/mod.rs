use std::time::{Duration, Instant};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Timeout(libc::c_int);

impl Timeout {
    pub const fn infinite() -> Self {
        Self(-1)
    }

    pub const fn millis(&self) -> i32 {
        self.0
    }

    pub const fn from_duration(timeout: Duration) -> Self {
        let time = timeout.as_millis();
        if time > i32::MAX as u128 {
            Timeout::infinite()
        } else {
            Timeout(time as i32)
        }
    }

    pub const fn into_duration(self) -> Duration {
        let millis = if self.0 == -1 {
            u32::MAX
        } else {
            self.0 as u32
        };
        Duration::from_millis(millis as u64)
    }
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Debug)]
pub struct Deadline(Instant);

impl Deadline {
    pub(crate) fn new(timeout: Timeout) -> Self {
        Self(Instant::now() + timeout.into_duration())
    }

    pub(super) fn now() -> Self {
        Deadline(Instant::now())
    }

    pub(super) fn elapsed(&self) -> Duration {
        self.0.elapsed()
    }

    #[cfg(feature = "timerfd")]
    pub(super) fn as_absolute_timespec(&self) -> libc::timespec {
        unimplemented!("")
    }
}

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use self::unix::{Socket, Fd, Signal};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::{Socket, AsRawHandle, Handle};
