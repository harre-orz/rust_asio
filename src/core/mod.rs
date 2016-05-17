use std::mem;
use std::cmp;
use std::time::Duration;
use time;
use libc;

macro_rules! libc_try {
    ($expr:expr) => (match unsafe { $expr } {
        rc if rc >= 0 => rc,
        _ => return Err(io::Error::last_os_error()),
    })
}

mod libc_ext {
    use libc::*;

    extern {
        #[cfg_attr(target_os = "linux", link_name = "__errno_location")]
        pub fn errno_location() -> *mut c_int;
    }
}

pub fn errno() -> i32 {
    unsafe { *libc_ext::errno_location() }
}


#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Expiry {
    expiry: Duration
}

impl Expiry {
    fn now() -> Expiry {
        Expiry {
            expiry: (time::SteadyTime::now() - time::SteadyTime::zero()).to_std().unwrap()
        }
    }

    fn default() -> Expiry {
        Expiry {
            expiry: Duration::new(0, 0)
        }
    }

    fn max_value() -> Expiry {
        Expiry {
            expiry: Duration::new(u64::max_value(), 0)
        }
    }

    fn wait_duration(&self, min: Duration) -> Duration {
        let dur = self.expiry - Self::now().expiry;
        cmp::min(dur, min)
    }

    pub fn wait_duration_msec(&self, min: Duration) -> u64 {
        let diff = self.wait_duration(min);
        diff.as_secs() * 1000 + diff.subsec_nanos() as u64 / 1000000
    }

    pub fn wait_duration_usec(&self, min: Duration) -> u64 {
        let diff = self.wait_duration(min);
        diff.as_secs() * 1000000 + diff.subsec_nanos() as u64 / 1000
    }

    pub fn wait_monotonic_timespec(&self) -> libc::timespec {
        libc::timespec {
            tv_sec: self.expiry.as_secs() as i64,
            tv_nsec: self.expiry.subsec_nanos() as i64,
        }
    }

    pub fn wait_monotonic_timeval(&self) -> libc::timeval {
        libc::timeval {
            tv_sec: self.expiry.as_secs() as i64,
            tv_usec: self.expiry.subsec_nanos() as i64 / 1000,
        }
    }
}

pub type NativeHandleType = libc::c_int;

pub trait ToExpiry {
    fn zero() -> Self;
    fn now() -> Self;
    fn to_expiry(self) -> Expiry;
}

impl ToExpiry for time::SteadyTime {
    fn zero() -> Self {
        unsafe { mem::zeroed() }
    }

    fn now() -> Self {
        time::SteadyTime::now()
    }

    fn to_expiry(self) -> Expiry {
        match (self - Self::zero()).to_std() {
            Ok(expiry)
                => Expiry{ expiry: expiry },
            _
                => Expiry::default(),
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

    fn to_expiry(self) -> Expiry {
        let now = Expiry::now().expiry;
        match (self - Self::now()).to_std() {
            Ok(expiry) if expiry > now
                => Expiry { expiry: expiry - now },
            _
                => Expiry::default(),
        }
    }
}

pub mod task_service;
pub mod timer_queue;
pub mod epoll_reactor;
