use core::{IoContext, AsIoContext, ThreadIoContext, Perform, Reactor, Intr};
use ffi::{SystemError, OPERATION_CANCELED};

use std::mem;
use std::cmp::Ordering;
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
    pub expiry: Expiry,
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
        let (old_op, update) = {
            let mut tq = this.as_ctx().0.tq.mutex.lock().unwrap();
            let old_op = self.op.take();
            self.op = Some(op);
            (old_op, TimerQueue::insert(&mut tq, self))
        };

        if let Some(expiry) = update {
            this.as_ctx().0.tq.reset_timeout(&this.as_ctx().0.intr, expiry)
        }

        if let Some(op) = old_op {
            this.push_back(op, OPERATION_CANCELED);
        }
    }

    pub fn cancel_timer_op(&mut self, ctx: &IoContext) {
        let (old_op, update) = {
            let mut tq = ctx.0.tq.mutex.lock().unwrap();
            let old_op = self.op.take();
            (old_op, TimerQueue::erase(&mut tq, self))
        };

        if let Some(expiry) = update {
            ctx.0.tq.reset_timeout(&ctx.0.intr, expiry)
        }

        if let Some(op) = old_op {
            ctx.do_perform(op, OPERATION_CANCELED)
        }
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


pub struct TimerQueue {
    mutex: Mutex<Vec<TimerOpPtr>>,
}

impl TimerQueue {
    pub fn new() -> Result<Self, SystemError> {
        let tq = TimerQueue {
            mutex: Mutex::default(),
        };
        Ok(tq)
    }

    pub fn startup(&self, reactor: &Reactor) {
    }

    pub fn cleanup(&self, reactor: &Reactor) {
    }

    fn insert(tq: &mut Vec<TimerOpPtr>, timer: &TimerOp) -> Option<Expiry> {
        if let Err(i) = tq.binary_search(&TimerOpPtr(timer)) {
            tq.insert(i, TimerOpPtr(timer));
            if i == 0 {
                return Some(timer.expiry.clone())
            }
        }
        None
    }

    fn erase(tq: &mut Vec<TimerOpPtr>, timer: &TimerOp) -> Option<Expiry> {
        if let Ok(i) = tq.binary_search(&TimerOpPtr(timer)) {
            tq.remove(i);
            if i == 0 {
                return Some(timer.expiry.clone())
            }
        }
        None
    }

    pub fn get_ready_timers(&self, this: &mut ThreadIoContext, now: Expiry) {
        let mut tq = self.mutex.lock().unwrap();
        let i = match tq.binary_search_by(|timer| timer.expiry.cmp(&now)) {
            Ok(i) => i + 1,
            Err(i) => i,
        };
        for mut timer in tq.drain(..i) {
            this.push_back(timer.op.take().unwrap(), SystemError::default());
        }
    }

    pub fn cancel_all_timers(&self, this: &mut ThreadIoContext) {
        let mut tq = self.mutex.lock().unwrap();
        for mut timer in tq.drain(..) {
            this.push_back(timer.op.take().unwrap(), OPERATION_CANCELED);
        }
    }

    pub fn wait_duration(&self) -> Option<Expiry> {
        let tq = self.mutex.lock().unwrap();
        if tq.is_empty() {
            None
        } else {
            Some(tq[0].expiry.clone())
        }
    }

    fn reset_timeout(&self, intr: &Intr, _: Expiry) {
        intr.interrupt()
    }
}
