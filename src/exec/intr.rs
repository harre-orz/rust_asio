#[cfg(target_os = "linux")]
mod timerfd;

#[cfg(target_os = "linux")]
pub use self::timerfd::TimerFd as Intr;

#[cfg(target_os = "macos")]
mod eventfd;

#[cfg(target_os = "macos")]
pub use self::eventfd::EventFd as Intr;

#[cfg(target_os = "windows")]
mod pipe;

#[cfg(target_os = "windows")]
use self::pipe::SocketPair as Intr;
