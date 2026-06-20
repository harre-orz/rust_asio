use crate::primitive::Timeout;
use std::cell::UnsafeCell;
use std::cmp::Ordering;
use std::mem::MaybeUninit;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub(in super::super) struct Deadline(UnsafeCell<Instant>);

impl Deadline {
    pub const UNINIT: Self = {
        let uninit = MaybeUninit::uninit();
        Self(UnsafeCell::new(unsafe { uninit.assume_init() }))
    };

    pub fn now() -> Self {
        Deadline(UnsafeCell::new(Instant::now()))
    }

    pub const fn clone(&self) -> Self {
        Self(UnsafeCell::new(unsafe { *self.0.get() }))
    }

    pub fn update(&self, t: Timeout) {
        unsafe { *self.0.get() = Instant::now() + Duration::from_millis(t.0 as u64) };
    }

    pub fn as_millis(&self) -> i32 {
        let tv = unsafe { *self.0.get() };
        tv.elapsed().as_millis() as i32
    }

    #[cfg(target_os = "macos")]
    pub fn as_timespec(&self) -> libc::timespec  {
        let tv = unsafe { *self.0.get() }.elapsed();
        libc::timespec {
            tv_sec: tv.as_secs() as libc::time_t,
            tv_nsec: tv.subsec_nanos() as libc::c_long,
        }
    }

    pub fn set(&self, deadline: Self) {
        unsafe { *self.0.get() = *deadline.0.get() };
    }
}

impl Eq for Deadline {}

impl PartialEq for Deadline {
    fn eq(&self, other: &Self) -> bool {
        unsafe { &*self.0.get() }.eq(unsafe { &*other.0.get() })
    }
}

impl Ord for Deadline {
    fn cmp(&self, other: &Self) -> Ordering {
        unsafe { &*self.0.get() }.cmp(unsafe { &*other.0.get() })
    }
}

impl PartialOrd for Deadline {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        unsafe { &*self.0.get() }.partial_cmp(unsafe { &*other.0.get() })
    }
}
