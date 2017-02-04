use core::Expiry;

use std::ops::Add;
use std::time::{Duration, SystemTime, Instant};

pub trait Clock : Send + 'static {
    type Duration;

    type TimePoint : Add<Self::Duration, Output = Self::TimePoint> + Into<Expiry>;

    fn now() -> Self::TimePoint;
}

/// Provides a monotonic clock.
pub struct SteadyClock;

impl Clock for SteadyClock {
    type Duration = Duration;

    type TimePoint = Instant;

    fn now() -> Self::TimePoint {
        Instant::now()
    }
}

/// Provides a real-time clock.
pub struct SystemClock;

impl Clock for SystemClock {
    type Duration = Duration;

    type TimePoint = SystemTime;

    fn now() -> Self::TimePoint {
        SystemTime::now()
    }
}
