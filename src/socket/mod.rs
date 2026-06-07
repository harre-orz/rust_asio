use std::time::Duration;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Timeout(libc::c_int);

impl Timeout {
    pub const fn infinite() -> Self {
        Self(-1)
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

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub(crate) use self::unix::Fd;
#[cfg(unix)]
pub use self::unix::Socket;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use self::windows::{startup, cleanup, WinSockEx};
#[cfg(windows)]
pub use self::windows::Socket;
