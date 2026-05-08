#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub(crate) use self::unix::{Fd, Timeout};
#[cfg(unix)]
pub use self::unix::{MAX_CONNECTIONS, Shutdown, Socket, SocketType};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use self::windows::Timeout;
#[cfg(windows)]
pub use self::windows::{MAX_CONNECTIONS, Shutdown, Socket, SocketType};
