use super::*;
use ffi::OPERATION_CANCELED;

use std::mem;
use std::ptr;
use std::cmp;
use std::cmp::Ordering;
use std::ops::{Deref, DerefMut};
use std::time::{Duration, Instant, SystemTime};
use std::sync::Mutex;

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
            (Ordering::Equal, Ordering::Greater) => {
                Duration::new(0, self.0.subsec_nanos() - other.0.subsec_nanos())
            }
            (Ordering::Greater, Ordering::Less) => {
                Duration::new(
                    self.0.as_secs() - other.0.as_secs() - 1,
                    1000000000 - (other.0.subsec_nanos() - self.0.subsec_nanos()),
                )
            }
            (Ordering::Greater, Ordering::Equal) => {
                Duration::new(self.0.as_secs() - other.0.as_secs(), 0)
            }
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

pub struct InnerTimer {
    ctx: IoContext,
    expiry: Expiry,
    op: Option<Box<Perform>>,
}

impl InnerTimer {
    pub fn new(ctx: &IoContext) -> Box<Self> {
        Box::new(InnerTimer {
            ctx: ctx.clone(),
            expiry: Expiry::zero(),
            op: None,
        })
    }

    pub fn cancel(&self) {
        let (old_op, update) = {
            let mut tq = self.ctx.as_reactor().tq.lock().unwrap();
            let old_op = unsafe { &mut *(self as *const _ as *mut Self) }.op.take();
            (old_op, erase(&mut tq, self))
        };
        if let Some(expiry) = update {
            self.ctx.as_reactor().reset_timeout(expiry)
        }
        if let Some(op) = old_op {
            self.ctx.do_dispatch((op, OPERATION_CANCELED))
        }
    }

    pub fn reset_expiry(&self, expiry: Expiry) {
        let (old_op, update) = {
            let mut tq = self.ctx.as_reactor().tq.lock().unwrap();
            let old_op = unsafe { &mut *(self as *const _ as *mut Self) }.op.take();
            let update = erase(&mut tq, self);
            unsafe { &mut *(self as *const _ as *mut Self) }.expiry = expiry;
            (old_op, update)
        };
        if let Some(expiry) = update {
            self.ctx.as_reactor().reset_timeout(expiry)
        }
        if let Some(op) = old_op {
            self.ctx.do_dispatch((op, OPERATION_CANCELED))
        }
    }

    pub fn set_wait_op(&self, this: &mut ThreadIoContext, op: Box<Perform>) {
        let (old_op, update) = {
            let mut tq = self.ctx.as_reactor().tq.lock().unwrap();
            let old_op = unsafe { &mut *(self as *const _ as *mut Self) }.op.take();
            unsafe { &mut *(self as *const _ as *mut Self) }.op = Some(op);
            (old_op, insert(&mut tq, self))
        };
        if let Some(expiry) = update {
            self.ctx.as_reactor().reset_timeout(expiry)
        }
        if let Some(op) = old_op {
            this.push(op, OPERATION_CANCELED)
        }
    }
}

unsafe impl AsIoContext for InnerTimer {
    fn as_ctx(&self) -> &IoContext {
        if let Some(this) = ThreadIoContext::callstack(&self.ctx) {
            this.as_ctx()
        } else {
            &self.ctx
        }
    }
}

impl Eq for InnerTimer {}

impl Ord for InnerTimer {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.expiry.cmp(&other.expiry) {
            Ordering::Equal => (self as *const _ as usize).cmp(&(other as *const _ as usize)),
            cmp => cmp,
        }
    }
}

impl PartialEq for InnerTimer {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self, other)
    }
}

impl PartialOrd for InnerTimer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.expiry.partial_cmp(&other.expiry) {
            Some(Ordering::Equal) => {
                (self as *const _ as usize).partial_cmp(&(other as *const _ as usize))
            }
            cmp => cmp,
        }
    }
}

pub struct InnerTimerPtr(pub *const InnerTimer);

impl Eq for InnerTimerPtr {}

impl Ord for InnerTimerPtr {
    fn cmp(&self, other: &Self) -> Ordering {
        unsafe { (&*self.0).cmp(&*other.0) }
    }
}

impl PartialEq for InnerTimerPtr {
    fn eq(&self, other: &Self) -> bool {
        unsafe { (&*self.0).eq(&*other.0) }
    }
}

impl PartialOrd for InnerTimerPtr {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        unsafe { (&*self.0).partial_cmp(&*other.0) }
    }
}

unsafe impl Send for InnerTimerPtr {}

impl Deref for InnerTimerPtr {
    type Target = InnerTimer;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

impl DerefMut for InnerTimerPtr {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self.0 as *mut InnerTimer) }
    }
}

pub type TimerQueue = Vec<InnerTimerPtr>;

fn insert(tq: &mut TimerQueue, timer: &InnerTimer) -> Option<Expiry> {
    if let Err(i) = tq.binary_search(&InnerTimerPtr(timer)) {
        tq.insert(i, InnerTimerPtr(timer));
        if i == 0 {
            return Some(timer.expiry.clone());
        }
    }
    None
}

fn erase(tq: &mut TimerQueue, timer: &InnerTimer) -> Option<Expiry> {
    if let Ok(i) = tq.binary_search(&InnerTimerPtr(timer)) {
        tq.remove(i);
        if i == 0 {
            return Some(timer.expiry.clone());
        }
    }
    None
}

pub fn get_ready_timers(tq: &mut TimerQueue, this: &mut ThreadIoContext, now: Expiry) {
    let i = match tq.binary_search_by(|e| e.expiry.cmp(&now)) {
        Ok(i) => i + 1,
        Err(i) => i,
    };
    for mut e in tq.drain(..i) {
        this.push(e.op.take().unwrap(), SystemError::default());
    }
}

pub fn wait_duration(tq: &Mutex<TimerQueue>, max: Duration) -> Duration {
    let tq = tq.lock().unwrap();
    if let Some(op) = tq.first() {
        cmp::min(max, op.expiry.left())
    } else {
        max
    }
}

#[test]
fn test_expiry_diff() {
    assert_eq!(
        Expiry(Duration::new(1, 1)).diff(Expiry(Duration::new(2, 0))),
        Duration::new(0, 0)
    );
    assert_eq!(
        Expiry(Duration::new(1, 1)).diff(Expiry(Duration::new(2, 1))),
        Duration::new(0, 0)
    );
    assert_eq!(
        Expiry(Duration::new(1, 1)).diff(Expiry(Duration::new(2, 2))),
        Duration::new(0, 0)
    );

    assert_eq!(
        Expiry(Duration::new(1, 1)).diff(Expiry(Duration::new(1, 2))),
        Duration::new(0, 0)
    );
    assert_eq!(
        Expiry(Duration::new(1, 1)).diff(Expiry(Duration::new(1, 1))),
        Duration::new(0, 0)
    );
    assert_eq!(
        Expiry(Duration::new(1, 1)).diff(Expiry(Duration::new(1, 0))),
        Duration::new(0, 1)
    );

    assert_eq!(
        Expiry(Duration::new(1, 1)).diff(Expiry(Duration::new(0, 0))),
        Duration::new(1, 1)
    );
    assert_eq!(
        Expiry(Duration::new(1, 1)).diff(Expiry(Duration::new(0, 1))),
        Duration::new(1, 0)
    );
    assert_eq!(
        Expiry(Duration::new(1, 1)).diff(Expiry(Duration::new(0, 2))),
        Duration::new(0, 999999999)
    );
}

#[test]
fn test_eq() {
    use std::time::Instant;
    let now = Instant::now();

    let ctx = &IoContext::new().unwrap();
    let t1 = InnerTimer {
        ctx: ctx.clone(),
        expiry: now.into(),
        op: None,
    };

    let t2 = InnerTimer {
        ctx: ctx.clone(),
        expiry: now.into(),
        op: None,
    };

    assert!(t1 == t1);
    assert!(t1 != t2);
}

#[test]
fn test_cmp() {
    use std::time::{Duration, Instant};
    let now = Instant::now();

    let ctx = &IoContext::new().unwrap();
    let t1 = InnerTimer {
        ctx: ctx.clone(),
        expiry: (now + Duration::new(1, 0)).into(),
        op: None,
    };

    let t2 = InnerTimer {
        ctx: ctx.clone(),
        expiry: (now + Duration::new(2, 0)).into(),
        op: None,
    };

    let t3 = InnerTimer {
        ctx: ctx.clone(),
        expiry: (now + Duration::new(2, 0)).into(),
        op: None,
    };

    assert!(t1 < t2);

    if (&t2 as *const _) < (&t3 as *const _) {
        assert!(t2 < t3);
    } else {
        assert!(t3 < t2);
    }
}
