#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub(crate) use self::unix::Fd;
#[cfg(unix)]
pub use self::unix::{Socket, SocketType};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::{Socket, SocketType};
