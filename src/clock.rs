use std::mem;
use std::time::{Duration, SystemTime, Instant};

/// タイマの満了時間(モノトニック時間).
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
#[doc(hidden)]
pub struct Expiry(Duration);

impl Expiry {
    pub fn now() -> Expiry {
        Instant::now().into_expiry()
    }

    pub fn as_secs(&self) -> u64 {
        self.0.as_secs()
    }

    pub fn subsec_nanos(&self) -> u32 {
        self.0.subsec_nanos()
    }
}

impl Default for Expiry {
    fn default() -> Expiry {
        Expiry(Duration::new(i64::max_value() as u64, 0))
    }
}

#[doc(hidden)]
pub trait IntoExpiry {
    fn into_expiry(self) -> Expiry;
}

impl IntoExpiry for Instant {
    fn into_expiry(self) -> Expiry {
        Expiry(self.duration_since(unsafe { mem::zeroed() }))
    }
}

impl IntoExpiry for SystemTime {
    fn into_expiry(self) -> Expiry {
        Expiry(Expiry::now().0 + self.elapsed().unwrap())
    }
}

pub trait Clock : Send + 'static {
    type Duration;
    type TimePoint;

    #[doc(hidden)]
    fn expires_at(timepoint: Self::TimePoint) -> Expiry;

    #[doc(hidden)]
    fn expires_from(duration: Self::Duration) -> Expiry;

    #[doc(hidden)]
    fn elapsed_at(timepoint: Self::TimePoint) -> Duration;

    #[doc(hidden)]
    fn elapsed_from(duration: Self::Duration) -> Duration;
}

pub struct SteadyClock;
impl Clock for SteadyClock {
    type Duration = Duration;
    type TimePoint = Instant;

    #[doc(hidden)]
    fn expires_at(timepoint: Self::TimePoint) -> Expiry {
        timepoint.into_expiry()
    }

    #[doc(hidden)]
    fn expires_from(duration: Self::Duration) -> Expiry {
        (Instant::now() + duration).into_expiry()
    }

    #[doc(hidden)]
    fn elapsed_at(timepoint: Self::TimePoint) -> Duration {
        timepoint.elapsed()
    }

    #[doc(hidden)]
    fn elapsed_from(duration: Self::Duration) -> Duration {
        duration
    }
}

pub struct SystemClock;
impl Clock for SystemClock {
    type Duration = Duration;
    type TimePoint = SystemTime;

    #[doc(hidden)]
    fn expires_at(timepoint: Self::TimePoint) -> Expiry {
        timepoint.into_expiry()
    }

    #[doc(hidden)]
    fn expires_from(duration: Self::Duration) -> Expiry {
        (Instant::now() + duration).into_expiry()
    }

    #[doc(hidden)]
    fn elapsed_at(timepoint: Self::TimePoint) -> Duration {
        timepoint.elapsed().unwrap()
    }

    #[doc(hidden)]
    fn elapsed_from(duration: Self::Duration) -> Duration {
        duration
    }
}

#[test]
fn test_expiry() {
    let a = Instant::now().into_expiry();
    let b = SystemTime::now().into_expiry();
    assert!((a.0.as_secs() - b.0.as_secs()) <= 1);
}
