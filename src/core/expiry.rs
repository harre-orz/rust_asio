use std::mem;
use std::cmp::Ordering;
use std::time::{Duration, Instant, SystemTime};

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct Expiry(Duration);

impl Expiry {
    pub fn zero() -> Self {
        Expiry(Duration::new(0, 0))
    }

    pub fn infinity() -> Self {
        Expiry(Duration::new(u64::max_value(), 0))
    }

    pub fn now() -> Self {
        Instant::now().into()
    }

    fn diff(&self, other: Self) -> Duration {
        let sec_cmp = self.0.as_secs().cmp(&other.0.as_secs());
        let nsec_cmp = self.0.subsec_nanos().cmp(&other.0.subsec_nanos());
        match (sec_cmp, nsec_cmp) {
            (Ordering::Equal, Ordering::Greater) => Duration::new(0, self.0.subsec_nanos() - other.0.subsec_nanos()),
            (Ordering::Greater, Ordering::Less) => {
                Duration::new(
                    self.0.as_secs() - other.0.as_secs() - 1,
                    1000000000 - (other.0.subsec_nanos() - self.0.subsec_nanos()),
                )
            }
            (Ordering::Greater, Ordering::Equal) => Duration::new(self.0.as_secs() - other.0.as_secs(), 0),
            (Ordering::Greater, Ordering::Greater) => {
                Duration::new(
                    self.0.as_secs() - other.0.as_secs(),
                    self.0.subsec_nanos() - other.0.subsec_nanos(),
                )
            }
            _ => Duration::new(0, 0),
        }
    }

    pub fn left(&self) -> Duration {
        self.diff(Expiry::now())
    }

    pub fn abs(&self) -> Duration {
        self.0
    }
}

impl From<Instant> for Expiry {
    fn from(t: Instant) -> Self {
        Expiry(t.duration_since(unsafe { mem::zeroed() }))
    }
}

impl From<SystemTime> for Expiry {
    fn from(t: SystemTime) -> Self {
        match t.duration_since(SystemTime::now()) {
            Ok(t) => Expiry(Expiry::now().0 + t),
            Err(_) => Expiry::now(),
        }
    }
}
