#[cfg(all(feature = "timerfd", any(target_os = "linux")))]
mod timerfd;
#[cfg(all(feature = "timerfd", any(target_os = "linux")))]
pub(super) use self::timerfd::TimerFd as Intr;

#[cfg(all(feature = "eventfd", any(target_os = "linux")))]
mod eventfd;
#[cfg(all(feature = "eventfd", any(target_os = "linux")))]
pub(super) use self::eventfd::EventFd as Intr;

#[cfg(any(
    target_os = "macos",
    not(any(windows, feature = "timerfd", feature = "eventfd"))
))]
mod unix_pipe;
#[cfg(any(
    target_os = "macos",
    not(any(windows, feature = "timerfd", feature = "eventfd"))
))]
pub(super) use self::unix_pipe::Pipe as Intr;

#[cfg(windows)]
mod win_pipe;
#[cfg(windows)]
use self::win_pipe::Pipe as Intr;
