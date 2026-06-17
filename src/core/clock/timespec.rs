use crate::primitive::Timeout;
use std::cmp;
use std::hash;
use std::mem::MaybeUninit;

#[derive(Copy, Clone, Debug)]
pub(in super::super) struct Deadline(libc::timespec);

impl Deadline {
    pub const UNSPECIFIED: Self = Self(libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    });

    pub fn now() -> Self {
        let mut tv = MaybeUninit::uninit();
        unsafe {
            libc::clock_gettime(libc::CLOCK_MONOTONIC, tv.as_mut_ptr());
            Self(tv.assume_init())
        }
    }

    pub fn new(timeout: Timeout) -> Self {
        let Self(mut tv) = Self::now();
        tv.tv_sec += timeout.0 as libc::c_long / 1_000;
        tv.tv_nsec += timeout.0 as libc::c_long * 1_000_000;
        if tv.tv_nsec > 1_000_000_000 {
            tv.tv_sec += 1;
            tv.tv_nsec %= 1_000_000_000;
        }
        Self(tv)
    }

    pub fn absolute(self) -> libc::timespec {
        self.0
    }
}

impl PartialEq for Deadline {
    fn eq(&self, other: &Self) -> bool {
        self.0.tv_sec == other.0.tv_sec && self.0.tv_nsec == other.0.tv_nsec
    }
}

impl Eq for Deadline {}

impl PartialOrd for Deadline {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        match self.0.tv_sec.partial_cmp(&other.0.tv_sec) {
            Some(cmp::Ordering::Equal) => self.0.tv_nsec.partial_cmp(&other.0.tv_nsec),
            ord => ord,
        }
    }
}

impl Ord for Deadline {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        match self.0.tv_sec.cmp(&other.0.tv_sec) {
            cmp::Ordering::Equal => self.0.tv_nsec.cmp(&other.0.tv_nsec),
            ord => ord,
        }
    }
}

impl hash::Hash for Deadline {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        state.write_i64(self.0.tv_sec);
        state.write_i64(self.0.tv_nsec);
    }
}
