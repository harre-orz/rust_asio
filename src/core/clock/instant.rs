use std::time::Instant;

#[derive(Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Debug)]
pub(in super::super) struct Deadline(Instant);

#[cfg(not(all(target_os = "linux", feature = "timerfd")))]
impl Deadline {
    pub fn now() -> Self {
        Deadline(Instant::now())
    }

    // pub(crate) fn new(timeout: Timeout) -> Self {
    //     if timeout == Timeout::INFINITE {
    //         Self(Instant::now() + Duration::from_secs(60 * 60 * 24 * 365 * 100))
    //     } else {
    //         Self(Instant::now() + Duration::from_millis(timeout.0 as u64))
    //     }
    // }
    //
    // pub(crate) fn elapsed(&self) -> Duration {
    //     self.0.elapsed()
    // }
}
