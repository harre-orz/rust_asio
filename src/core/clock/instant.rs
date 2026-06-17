use crate::primitive::Timeout;
use std::time::{Duration, Instant};

#[derive(Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Debug)]
pub(in super::super) struct Deadline(Instant);

#[cfg(not(all(target_os = "linux", feature = "timerfd")))]
impl Deadline {
    pub fn now() -> Self {
        Deadline(Instant::now())
    }

    pub(crate) fn new(t: Timeout) -> Self {
        Self(Instant::now() + Duration::from_millis(t.0 as u64))
    }

    pub(crate) fn elapsed(&self) -> Duration {
        self.0.elapsed()
    }
}
