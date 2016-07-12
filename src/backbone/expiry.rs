use std::mem;
use std::time::Duration;
use time;
use ops::*;

#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Expiry(Duration);

impl Expiry {
    pub fn now() -> Expiry {
        Expiry((time::SteadyTime::now() - time::SteadyTime::zero()).to_std().unwrap())
    }

    pub fn default() -> Expiry {
        Expiry(Duration::new(0, 0))
    }

    pub fn max_value() -> Expiry {
        Expiry(Duration::new(u64::max_value(), 0))
    }

    pub fn wait_duration(&self) -> Duration {
        self.0 - Self::now().0
    }

    pub fn wait_monotonic_timespec(&self) -> timespec {
        timespec {
            tv_sec: self.0.as_secs() as i64,
            tv_nsec: self.0.subsec_nanos() as i64,
        }
    }
}

pub trait ToExpiry {
    fn zero() -> Self;
    fn now() -> Self;
    fn to_expiry(&self) -> Expiry;
}

impl ToExpiry for time::SteadyTime {
    fn zero() -> Self {
        unsafe { mem::zeroed() }
    }

    fn now() -> Self {
        time::SteadyTime::now()
    }

    fn to_expiry(&self) -> Expiry {
        match (*self - Self::zero()).to_std() {
            Ok(expiry) => Expiry(expiry),
            _          => Expiry::default(),
        }
    }
}

impl ToExpiry for time::Tm {
    fn zero() -> Self {
        time::empty_tm()
    }

    fn now() -> Self {
        time::now()
    }

    fn to_expiry(&self) -> Expiry {
        let now = Expiry::now().0;
        match (*self - Self::now()).to_std() {
            Ok(expiry) if expiry > now  => Expiry(expiry - now),
            _                           => Expiry::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;
    use time;

    #[bench]
    fn bench_system_time_now(b: &mut Bencher) {
        b.iter(|| {
            let _ = time::now();
        });
    }

    #[bench]
    fn bench_system_time_to_expiry(b: &mut Bencher) {
        let t = time::now();
        b.iter(|| {
            let _ = t.to_expiry();
        });
    }

    #[bench]
    fn bench_steady_time_now(b: &mut Bencher) {
        b.iter(|| {
            let _ = time::SteadyTime::now();
        });
    }

    #[bench]
    fn bench_steady_time_to_expiry(b: &mut Bencher) {
        let t = time::SteadyTime::now();
        b.iter(|| {
            let _ = t.to_expiry();
        });
    }
}
