use crate::primitive::Timeout;
use std::cell::UnsafeCell;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::mem::MaybeUninit;

fn now() -> libc::timespec {
    let mut tv = MaybeUninit::uninit();
    unsafe {
        let clk_id = libc::CLOCK_MONOTONIC;
        libc::clock_gettime(clk_id, tv.as_mut_ptr());
        tv.assume_init()
    }
}

#[derive(Debug)]
pub(in super::super) struct Deadline(UnsafeCell<libc::timespec>);

impl Deadline {
    pub const fn dangling() -> Self {
        let uninit = MaybeUninit::uninit();
        Self(UnsafeCell::new(unsafe { uninit.assume_init() }))
    }

    pub fn now() -> Self {
        Self(UnsafeCell::new(now()))
    }

    pub fn update(&self, t: Timeout) {
        let mut tv = now();
        tv.tv_sec += t.0 as libc::c_long / 1_000;
        tv.tv_nsec += t.0 as libc::c_long * 1_000_000;
        if tv.tv_nsec > 1_000_000_000 {
            tv.tv_sec += 1;
            tv.tv_nsec %= 1_000_000_000;
        }
        unsafe { *self.0.get() = tv };
    }

    #[cfg(feature = "timerfd")]
    pub fn absolute(&self) -> libc::timespec {
        unsafe { *self.0.get() }
    }

    #[cfg(not(feature = "timerfd"))]
    pub fn clone(&self) -> Self {
        Self(UnsafeCell::new(unsafe { *self.0.get() }))
    }

    pub const fn as_millis(&self) -> i32 {
        let tv = unsafe { *self.0.get() };
        (tv.tv_nsec / 1_000_000) as i32 + (tv.tv_sec * 1_000) as i32
    }
}

impl PartialEq for Deadline {
    fn eq(&self, other: &Self) -> bool {
        let l = unsafe { &*self.0.get() };
        let r = unsafe { &*other.0.get() };
        l.tv_sec == r.tv_sec && l.tv_nsec == r.tv_nsec
    }
}

impl Eq for Deadline {}

impl PartialOrd for Deadline {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let l = unsafe { &*self.0.get() };
        let r = unsafe { &*other.0.get() };
        match l.tv_sec.partial_cmp(&r.tv_sec) {
            Some(Ordering::Equal) => l.tv_nsec.partial_cmp(&r.tv_nsec),
            ord => ord,
        }
    }
}

impl Ord for Deadline {
    fn cmp(&self, other: &Self) -> Ordering {
        let l = unsafe { &*self.0.get() };
        let r = unsafe { &*other.0.get() };
        match l.tv_sec.cmp(&r.tv_sec) {
            Ordering::Equal => l.tv_nsec.cmp(&r.tv_nsec),
            ord => ord,
        }
    }
}

impl Hash for Deadline {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let l = unsafe { &*self.0.get() };
        state.write_i64(l.tv_sec);
        state.write_i64(l.tv_nsec);
    }
}

unsafe impl Sync for Deadline {}
