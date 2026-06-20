#[cfg(any(
    all(target_os = "linux", feature = "timerfd"),
    all(target_os = "macos", feature = "ktimer"),
))]
mod timespec;
#[cfg(any(
    all(target_os = "linux", feature = "timerfd"),
    all(target_os = "macos", feature = "ktimer"),
))]
pub(super) use self::timespec::Deadline;

#[cfg(not(any(
    all(target_os = "linux", feature = "timerfd"),
    all(target_os = "macos", feature = "ktimer"),
)))]
mod instant;
#[cfg(not(any(
    all(target_os = "linux", feature = "timerfd"),
    all(target_os = "macos", feature = "ktimer"),
)))]
pub(super) use self::instant::Deadline;
