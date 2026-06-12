use super::Deadline;

#[cfg(all(feature = "timerfd", any(target_os = "linux")))]
mod intr_timerfd;
#[cfg(all(feature = "timerfd", any(target_os = "linux")))]
pub(crate) use self::intr_timerfd::TimerFd as Intr;

#[cfg(all(feature = "eventfd", any(target_os = "linux")))]
mod intr_eventfd;
#[cfg(all(feature = "eventfd", any(target_os = "linux")))]
use self::intr_eventfd::EventFd as Intr;

#[cfg(any(
    target_os = "macos",
    not(any(windows, feature = "timerfd", feature = "eventfd"))
))]
mod intr_pipe_unix;
#[cfg(any(
    target_os = "macos",
    not(any(windows, feature = "timerfd", feature = "eventfd"))
))]
pub(crate) use self::intr_pipe_unix::Pipe as Intr;

#[cfg(windows)]
mod intr_pipe_win;
#[cfg(windows)]
use self::intr_pipe_win::Pipe as Intr;
