#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use self::unix::Socket;
#[cfg(unix)]
pub(crate) use self::unix::{Fd, SocketType};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::Socket;
#[cfg(windows)]
pub(crate) use self::windows::SocketType;
