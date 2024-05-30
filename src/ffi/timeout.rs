use libc;
use std::cmp;
use std::fmt;
use std::mem::MaybeUninit;
use std::ops::Add;
use std::time::Duration;

#[derive(PartialEq, Eq, Ord, PartialOrd, Clone, Copy)]
pub(crate) struct Timeout {
    millis: u32,
}

impl Timeout {
    pub const fn new() -> Self {
        Self { millis: u32::MAX }
    }

    pub const fn as_nanos(self) -> u64 {
        self.millis as u64 * 1_000_000
    }

    pub const fn as_millis_i32(self) -> i32 {
        if self.millis > i32::MAX as u32 {
            -1
        } else {
            self.millis as i32
        }
    }
}

impl From<Duration> for Timeout {
    fn from(timeout: Duration) -> Self {
        let millis = timeout.as_millis();
        let millis = if millis > u32::MAX as u128 {
            u32::MAX
        } else {
            millis as u32
        };
        Self { millis: millis }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Monotonic {
    tv: libc::timespec,
}

impl Monotonic {
    pub fn now() -> Self {
        let mut tv = MaybeUninit::<libc::timespec>::uninit();
        unsafe {
            match libc::clock_gettime(libc::CLOCK_MONOTONIC, tv.as_mut_ptr()) {
                0 => Self {
                    tv: tv.assume_init(),
                },
                _ => panic!(),
            }
        }
    }

    pub fn into_itimerspec(self) -> libc::itimerspec {
        libc::itimerspec {
            it_interval: libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            it_value: self.tv,
        }
    }

    fn timeout_at(&self, now: Monotonic) -> Timeout {
        if self.tv.tv_sec < now.tv.tv_sec {
            return Timeout { millis: 0 };
        }
        let mut sec = self.tv.tv_sec - now.tv.tv_sec;
        let mut msec = (self.tv.tv_nsec - now.tv.tv_nsec) / 1_000_000;
        if msec < 0 {
            if sec == 0 {
                return Timeout { millis: 0 };
            } else {
                sec -= 1;
                msec += 1_000;
            }
        }
        let millis = if sec > i32::MAX as i64 {
            u32::MAX
        } else {
            sec as u32 * 1000 + msec as u32
        };

        Timeout { millis }
    }

    pub fn timeout_at_now(&self) -> Timeout {
        self.timeout_at(Self::now())
    }
}

impl cmp::Eq for Monotonic {}

impl cmp::PartialEq for Monotonic {
    fn eq(&self, other: &Self) -> bool {
        self.tv.tv_sec == other.tv.tv_sec && self.tv.tv_nsec == other.tv.tv_nsec
    }
}

impl cmp::Ord for Monotonic {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        match self.tv.tv_sec.cmp(&other.tv.tv_sec) {
            cmp::Ordering::Equal => self.tv.tv_nsec.cmp(&other.tv.tv_nsec),
            cmp => cmp,
        }
    }
}

impl cmp::PartialOrd for Monotonic {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(&other))
    }
}

impl fmt::Debug for Monotonic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{:09}", self.tv.tv_sec, self.tv.tv_nsec)
    }
}

impl Add<Timeout> for Monotonic {
    type Output = Self;

    fn add(self, rhs: Timeout) -> Self {
        let nsec = self.tv.tv_nsec as u64 + rhs.as_nanos();
        Self {
            tv: libc::timespec {
                tv_sec: self.tv.tv_sec + (nsec / 1_000_000_000) as i64,
                tv_nsec: (nsec % 1_000_000_000) as i64,
            },
        }
    }
}

impl Add<Duration> for Monotonic {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self {
        self + Timeout::from(rhs)
    }
}

#[test]
fn test_timeout_at_1s() {
    let now = Monotonic::now();
    let t = now + Duration::new(1, 0);
    assert_eq!(t.timeout_at(now).as_millis_i32(), 1000);
    assert_eq!(now.timeout_at(t).as_millis_i32(), 0);
}

#[test]
fn test_timeout_at_overflow() {
    let now = Monotonic::now();
    let t = Monotonic {
        tv: libc::timespec {
            tv_sec: i64::MAX,
            tv_nsec: 999_999_999,
        },
    };
    assert_eq!(t.timeout_at(now).as_millis_i32(), -1);
    assert_eq!(now.timeout_at(t).as_millis_i32(), 0);
}
