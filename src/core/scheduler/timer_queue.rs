use unsafe_cell::UnsafeBoxedCell;
use error::{READY, ECANCELED};
use core::{IoContext, AsIoContext, ThreadIoContext, Operation};

use std::mem;
use std::cmp::{Ordering};
use std::time::{Duration, SystemTime, Instant};

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
        match (
            self.0.as_secs().cmp(&other.0.as_secs()),
            self.0.subsec_nanos().cmp(&other.0.subsec_nanos())
        ){
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


pub struct TimerContext {
    pub ctx: IoContext,
    pub op: Option<Operation>,
    pub expiry: Expiry,
}

impl TimerContext {
    fn as_ptr(&self) -> *const Self {
        self
    }
}

impl Eq for TimerContext {
}

impl PartialEq for TimerContext {
    fn eq(&self, other: &Self) -> bool {
        self.expiry.eq(&other.expiry) && self.as_ptr().eq(&other.as_ptr())
    }
}

impl Ord for TimerContext {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.expiry.cmp(&other.expiry) {
            Ordering::Equal => self.as_ptr().cmp(&other.as_ptr()),
            cmp => cmp,
        }
    }
}

impl PartialOrd for TimerContext {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.expiry.partial_cmp(&other.expiry) {
            Some(Ordering::Equal) => self.as_ptr().partial_cmp(&other.as_ptr()),
            cmp => cmp,
        }
    }
}

pub struct AsyncTimer(UnsafeBoxedCell<TimerContext>);

impl AsyncTimer {
    pub fn new(ctx: &IoContext) -> Self {
        AsyncTimer(UnsafeBoxedCell::new(TimerContext {
            ctx: ctx.clone(),
            op: None,
            expiry: Expiry::zero(),
        }))
    }

    pub fn set_timer_op(&self, this: &mut ThreadIoContext, op: Operation) {
        if let Some(old_op) = self.as_ctx().0.scheduler.timer_queue_insert(self.0.clone(), op) {
            this.push(old_op, ECANCELED)
        }
    }

    pub fn set_expire_time(&self, this: &mut ThreadIoContext, expiry: Expiry) {
        if let Some(old_op) = self.as_ctx().0.scheduler.timer_queue_remove(self.0.clone()) {
            this.push(old_op, ECANCELED);
        }
        self.0.clone().expiry = expiry;
    }

}

unsafe impl AsIoContext for AsyncTimer {
    fn as_ctx(&self) -> &IoContext {
        &self.0.ctx
    }
}

#[derive(Default)]
pub struct TimerQueue(Vec<UnsafeBoxedCell<TimerContext>>);

impl TimerQueue {
    pub fn insert(&mut self, timer: &UnsafeBoxedCell<TimerContext>) -> Option<Expiry> {
        if let Err(idx) = self.0.binary_search(timer) {
            self.0.insert(idx, timer.clone());
            if idx == 0 {
                return Some(timer.expiry.clone());
            }
        }
        None
    }

    pub fn erase(&mut self, timer: &UnsafeBoxedCell<TimerContext>) -> Option<Expiry> {
        if let Ok(idx) = self.0.binary_search(timer) {
            self.0.remove(idx);
            if idx == 0 {
                return Some(timer.expiry.clone());
            }
        }
        None
    }

    pub fn get_ready_timers(&mut self, this: &mut ThreadIoContext, now: Expiry) {
        let idx = match self.0.binary_search_by(|e| e.expiry.cmp(&now)) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        };
        for mut timer in self.0.drain(..idx) {
            this.push(timer.op.take().unwrap(), READY);
        }
    }

    pub fn cancel_all_timers(&mut self, this: &mut ThreadIoContext) {
        let len = self.0.len();
        for mut timer in self.0.drain(..len) {
            this.push(timer.op.take().unwrap(), ECANCELED);
        }
    }

    pub fn front(&self) -> Option<Expiry> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0[0].expiry.clone())
        }
    }
}

#[test]
fn test_expiry_diff() {
    assert_eq!(Expiry(Duration::new(1,1)).diff(Expiry(Duration::new(2,0))),Duration::new(0,0));
    assert_eq!(Expiry(Duration::new(1,1)).diff(Expiry(Duration::new(2,1))),Duration::new(0,0));
    assert_eq!(Expiry(Duration::new(1,1)).diff(Expiry(Duration::new(2,2))),Duration::new(0,0));

    assert_eq!(Expiry(Duration::new(1,1)).diff(Expiry(Duration::new(1,2))),Duration::new(0,0));
    assert_eq!(Expiry(Duration::new(1,1)).diff(Expiry(Duration::new(1,1))),Duration::new(0,0));
    assert_eq!(Expiry(Duration::new(1,1)).diff(Expiry(Duration::new(1,0))),Duration::new(0,1));

    assert_eq!(Expiry(Duration::new(1,1)).diff(Expiry(Duration::new(0,0))),Duration::new(1,1));
    assert_eq!(Expiry(Duration::new(1,1)).diff(Expiry(Duration::new(0,1))),Duration::new(1,0));
    assert_eq!(Expiry(Duration::new(1,1)).diff(Expiry(Duration::new(0,2))),Duration::new(0,999999999));
}

#[test]
fn test_eq() {
    let ctx = &IoContext::new().unwrap();
    let t1 = TimerContext {
        ctx: ctx.clone(),
        op: None,
        expiry: Expiry(Duration::new(1,0)),
    };

    let t2 = TimerContext {
        ctx: ctx.clone(),
        op: None,
        expiry: Expiry(Duration::new(1,0)),
    };

    assert!(t1 == t1);
    assert!(t1 != t2);
}

#[test]
fn test_cmp() {
    let ctx = &IoContext::new().unwrap();
    let t1 = TimerContext {
        ctx: ctx.clone(),
        op: None,
        expiry: Expiry(Duration::new(1,0)),
    };

    let t2 = TimerContext {
        ctx: ctx.clone(),
        op: None,
        expiry: Expiry(Duration::new(2,0)),
    };

    let t3 = TimerContext {
        ctx: ctx.clone(),
        op: None,
        expiry: Expiry(Duration::new(2,0)),
    };

    assert!(t1 < t2);

    if t2.as_ptr() < t3.as_ptr() {
        assert!(t2 < t3);
    } else {
        assert!(t3 < t2);
    }
}
