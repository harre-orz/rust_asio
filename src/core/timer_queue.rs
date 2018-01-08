use core::{IoContext, AsIoContext, ThreadIoContext, Perform};
use ffi::{SystemError, OPERATION_CANCELED};

use std::mem;
use std::cmp::{Ordering};
use std::time::{Duration, SystemTime, Instant};
use std::ops::{Deref, DerefMut};
use std::sync::{Mutex};


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
            (Ordering::Equal, Ordering::Greater) =>
                Duration::new(
                    0,
                    self.0.subsec_nanos() - other.0.subsec_nanos()
                ),
            (Ordering::Greater, Ordering::Less) =>
                Duration::new(
                    self.0.as_secs() - other.0.as_secs() - 1,
                    1000000000 - (other.0.subsec_nanos() - self.0.subsec_nanos())
                ),
            (Ordering::Greater, Ordering::Equal) =>
                Duration::new(
                    self.0.as_secs() - other.0.as_secs(),
                    0
                ),
            (Ordering::Greater, Ordering::Greater) =>
                Duration::new(
                    self.0.as_secs() - other.0.as_secs(),
                    self.0.subsec_nanos() - other.0.subsec_nanos()
                ),
            _ =>
                Duration::new(0,0),
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


pub struct TimerOp {
    expiry: Expiry,
    op: Option<Box<Perform>>,
}

impl TimerOp {
    pub fn new() -> Self {
        TimerOp {
            expiry: Expiry::zero(),
            op: None,
        }
    }

    pub fn set_timer_op(&mut self, this: &mut ThreadIoContext, op: Box<Perform>) {
    }

    pub fn cancel_timer_op(&mut self, ctx: &IoContext) {
    }
}


struct TimerOpPtr(*const TimerOp);

impl PartialEq for TimerOpPtr {
    fn eq(&self, other: &Self) -> bool {
        unsafe { &*self.0 }.expiry.eq(&unsafe { &* other.0 }.expiry) && self.0.eq(&other.0)
    }
}

impl Eq for TimerOpPtr {
}

impl PartialOrd for TimerOpPtr {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match unsafe { &*self.0 }.expiry.partial_cmp(&unsafe { &*other.0 }.expiry) {
            Some(Ordering::Equal) => self.0.partial_cmp(&other.0),
            cmp => cmp,
        }
    }
}

impl Ord for TimerOpPtr {
    fn cmp(&self, other: &Self) -> Ordering {
        match unsafe { &*self.0 }.expiry.cmp(&unsafe { &*other.0 }.expiry) {
            Ordering::Equal => self.0.cmp(&other.0),
            cmp => cmp,
        }
    }
}

impl Deref for TimerOpPtr {
    type Target = TimerOp;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

impl DerefMut for TimerOpPtr {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self.0 as *mut _) }
    }
}


pub struct TimerQueue(Vec<TimerOpPtr>);

impl TimerQueue {
    pub fn insert(&mut self, timer: &TimerOp) -> Option<Expiry> {
        if let Err(i) = self.0.binary_search(&TimerOpPtr(timer)) {
            self.0.insert(i, TimerOpPtr(timer));
            if i == 0 {
                return Some(timer.expiry.clone())
            }
        }
        None
    }

    pub fn erase(&mut self, timer: &TimerOp) -> Option<Expiry> {
        if let Ok(i) = self.0.binary_search(&TimerOpPtr(timer)) {
            self.0.remove(i);
            if i == 0 {
                return Some(timer.expiry.clone())
            }
        }
        None
    }

    pub fn get_ready_timers(&mut self, this: &mut ThreadIoContext, now: Expiry) {
        let i = match self.0.binary_search_by(|timer| timer.expiry.cmp(&now)) {
            Ok(i) => i + 1,
            Err(i) => i,
        };
        for mut timer in self.0.drain(..i) {
            this.push_back(timer.op.take().unwrap(), SystemError::default());
        }
    }

    pub fn cancel_all_timers(&mut self, this: &mut ThreadIoContext) {
        for mut timer in self.0.drain(..) {
            this.push_back(timer.op.take().unwrap(), OPERATION_CANCELED);
        }
    }

    pub fn wait_duration(&self) -> Option<Expiry> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0[0].expiry.clone())
        }
    }
}
