use std::cell::Cell;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;

pub struct DurationOverflowError;

#[derive(Copy, Clone)]
pub(crate) struct Timeout(pub(crate) i32);

#[derive(Clone)]
pub(crate) struct AtomicTimeout(Cell<i32>);

impl AtomicTimeout {
    pub fn new(value: &AtomicI32) -> Self {
        Self(Cell::new(value.load(Ordering::Relaxed)))
    }

    pub fn set(&self, timeout: Duration) -> Result<(), DurationOverflowError> {
        let timeout = timeout.as_millis();
        if timeout > i32::MAX as u128 {
            Err(DurationOverflowError)
        } else {
            self.0.set(timeout as i32);
            Ok(())
        }
    }

    pub fn get(&self) -> Timeout {
        Timeout(self.0.get())
    }
}

unsafe impl Sync for AtomicTimeout {}

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
