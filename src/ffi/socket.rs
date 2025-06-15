#[cfg(unix)]
mod unix;

#[cfg(unix)]
pub use self::unix::{Fd, Socket};

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use self::windows::Socket;
