use super::Deadline;

#[cfg(all(target_os = "linux", not(any(feature = "eventfd", feature = "pipe"))))]
mod timerfd;
#[cfg(all(target_os = "linux", not(any(feature = "eventfd", feature = "pipe"))))]
pub(super) use self::timerfd::TimerFd as Intr;

#[cfg(all(target_os = "linux", feature = "eventfd"))]
mod eventfd;
#[cfg(all(target_os = "linux", feature = "eventfd"))]
pub(super) use self::eventfd::EventFd as Intr;

#[cfg(all(unix, feature = "pipe"))]
mod pipe_unix;
#[cfg(all(unix, feature = "pipe"))]
pub(super) use self::pipe_unix::Pipe as Intr;

#[cfg(windows)]
mod pipe_win;
#[cfg(windows)]
pub(super) use self::pipe_win::Pipe as Intr;
