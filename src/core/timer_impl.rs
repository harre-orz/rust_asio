use ffi::{SystemError, OPERATION_CANCELED};
use core::{IoContext, AsIoContext, ThreadIoContext, Perform, UnsafeRef};

use std::mem;
use std::ptr;
use std::cmp;
use std::cmp::Ordering;
use std::time::{Duration, Instant, SystemTime};
use std::sync::Mutex;

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct Expiry(Duration);

impl Expiry {
    fn zero() -> Self {
        Expiry(Duration::new(0, 0))
    }

    fn now() -> Self {
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

    fn left(&self) -> Duration {
        self.diff(Expiry::now())
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

pub struct TimerImpl {
    ctx: IoContext,
    expiry: Expiry,
    op: Option<Box<Perform>>,
}

impl TimerImpl {
    pub fn new(ctx: &IoContext) -> Box<Self> {
        Box::new(TimerImpl {
            ctx: ctx.clone(),
            expiry: Expiry::zero(),
            op: None,
        })
    }

    pub fn set_wait_op(&self, this: &mut ThreadIoContext, op: Box<Perform>) {
        let (old_op, update) = self.ctx.as_timer_queue().insert(self, op);
        if let Some(expiry) = update {
            self.ctx.as_reactor().reset_timeout(expiry)
        }
        if let Some(op) = old_op {
            this.push(op, OPERATION_CANCELED)
        }
    }

    pub fn reset_expiry(&self, expiry: Expiry) {
        let (old_op, update) = self.ctx.as_timer_queue().erase(self, expiry);
        if let Some(expiry) = update {
            self.ctx.as_reactor().reset_timeout(expiry)
        }
        if let Some(op) = old_op {
            self.ctx.do_dispatch((op, OPERATION_CANCELED))
        }
    }

    pub fn cancel(&self) {
        let (old_op, update) = self.ctx.as_timer_queue().erase(self, Expiry::zero());
        if let Some(expiry) = update {
            self.ctx.as_reactor().reset_timeout(expiry)
        }
        if let Some(op) = old_op {
            self.ctx.do_dispatch((op, OPERATION_CANCELED))
        }
    }
}

unsafe impl AsIoContext for TimerImpl {
    fn as_ctx(&self) -> &IoContext {
        if let Some(this) = ThreadIoContext::callstack(&self.ctx) {
            this.as_ctx()
        } else {
            &self.ctx
        }
    }
}

impl Eq for TimerImpl {}

impl Ord for TimerImpl {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.expiry.cmp(&other.expiry) {
            Ordering::Equal => (self as *const _ as usize).cmp(&(other as *const _ as usize)),
            cmp => cmp,
        }
    }
}

impl PartialEq for TimerImpl {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self, other)
    }
}

impl PartialOrd for TimerImpl {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.expiry.partial_cmp(&other.expiry) {
            Some(Ordering::Equal) => {
                (self as *const _ as usize).partial_cmp(&(other as *const _ as usize))
            }
            cmp => cmp,
        }
    }
}

type TimerRef = UnsafeRef<TimerImpl>;

pub struct TimerQueue(Mutex<Vec<TimerRef>>);

impl TimerQueue {
    pub fn new() -> Self {
        TimerQueue(Mutex::default())
    }

    pub fn wait_duration(&self, max: Duration) -> Duration {
        let tq = self.0.lock().unwrap();
        tq.first().map_or(max, |op| cmp::min(max, op.expiry.left()))
    }

    pub fn get_ready_timers(&self, this: &mut ThreadIoContext) {
        let mut tq = self.0.lock().unwrap();
        let i = match tq.binary_search_by(|e| e.expiry.cmp(&Expiry::now())) {
            Ok(i) => i + 1,
            Err(i) => i,
        };
        for mut e in tq.drain(..i) {
            this.push(e.op.take().unwrap(), SystemError::default());
        }
    }

    pub fn insert(
        &self,
        timer: &TimerImpl,
        op: Box<Perform>,
    ) -> (Option<Box<Perform>>, Option<Expiry>) {
        let mut timer = TimerRef::new(timer);
        let mut tq = self.0.lock().unwrap();
        let old_op = timer.op.take();
        timer.op = Some(op);
        let i = tq.binary_search(&timer).unwrap_err();
        tq.insert(i, unsafe { timer.clone() });
        let first = if i == 0 {
            Some(timer.expiry.clone())
        } else {
            None
        };
        (old_op, first)
    }

    pub fn erase(
        &self,
        timer: &TimerImpl,
        expiry: Expiry,
    ) -> (Option<Box<Perform>>, Option<Expiry>) {
        let mut timer = TimerRef::new(timer);
        let mut tq = self.0.lock().unwrap();
        let old_op = timer.op.take();
        let first = {
            if let Ok(i) = tq.binary_search(&timer) {
                tq.remove(i);
                tq.first().map(|timer| timer.expiry.clone())
            } else {
                None
            }
        };
        timer.expiry = expiry;
        (old_op, first)
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
    let t1 = TimerImpl {
        ctx: ctx.clone(),
        expiry: now.into(),
        op: None,
    };

    let t2 = TimerImpl {
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
    let t1 = TimerImpl {
        ctx: ctx.clone(),
        expiry: (now + Duration::new(1, 0)).into(),
        op: None,
    };

    let t2 = TimerImpl {
        ctx: ctx.clone(),
        expiry: (now + Duration::new(2, 0)).into(),
        op: None,
    };

    let t3 = TimerImpl {
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
