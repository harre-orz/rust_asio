use super::Deadline;

#[cfg(all(target_os = "linux", feature = "timerfd"))]
mod timerfd;
#[cfg(all(target_os = "linux", feature = "timerfd"))]
pub(super) use self::timerfd::TimerFd as Intr;

#[cfg(all(
    target_os = "linux",
    all(feature = "eventfd", not(feature = "timerfd"))
))]
mod eventfd;
#[cfg(all(
    target_os = "linux",
    all(feature = "eventfd", not(feature = "timerfd"))
))]
pub(super) use self::eventfd::EventFd as Intr;

#[cfg(all(target_os = "macos", feature = "ktimer"))]
mod ktimer;
#[cfg(all(target_os = "macos", feature = "ktimer"))]
pub(super) use self::ktimer::Ktimer as Intr;

#[cfg(not(any(
    windows,
    all(target_os = "linux", feature = "timerfd"),
    all(target_os = "linux", feature = "eventfd"),
    all(target_os = "macos", feature = "ktimer"),
)))]
mod pipe_unix;
#[cfg(not(any(
    windows,
    all(target_os = "linux", feature = "timerfd"),
    all(target_os = "linux", feature = "eventfd"),
    all(target_os = "macos", feature = "ktimer"),
)))]
pub(super) use self::pipe_unix::Pipe as Intr;

#[cfg(windows)]
mod pipe_win;
#[cfg(windows)]
pub(super) use self::pipe_win::Pipe as Intr;
