
#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use self::unix::{Fd, Signal};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use self::windows::{AsRawHandle, Handle};
