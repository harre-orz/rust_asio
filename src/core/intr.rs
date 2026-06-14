#[cfg(all(feature = "timerfd", any(target_os = "linux")))]
mod timerfd;
#[cfg(all(feature = "timerfd", any(target_os = "linux")))]
pub(super) use self::timerfd::TimerFd as Intr;

#[cfg(all(feature = "timerfd", any(target_os = "macos")))]
mod ktimer;
#[cfg(all(feature = "timerfd", any(target_os = "macos")))]
pub(super) use self::ktimer::Ktimer as Intr;

#[cfg(all(feature = "eventfd", any(target_os = "linux")))]
mod eventfd;
#[cfg(all(feature = "eventfd", any(target_os = "linux")))]
pub(super) use self::eventfd::EventFd as Intr;

#[cfg(not(any(windows, feature = "timerfd", feature = "eventfd")))]
mod pipe_unix;
#[cfg(not(any(windows, feature = "timerfd", feature = "eventfd")))]
pub(super) use self::pipe_unix::Pipe as Intr;

#[cfg(windows)]
mod pipe_win;
#[cfg(windows)]
pub(super) use self::pipe_win::Pipe as Intr;
