use ffi::{OPERATION_CANCELED};
use core::{IoContext, AsIoContext, ThreadIoContext, Perform};

use std::cmp::{Ordering};
use std::time::{Duration, Instant, SystemTime};

use libc::{timespec};

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct Expiry(Duration);

impl Expiry {
    pub fn zero() -> Self {
        Expiry(Duration::new(0, 0))
    }

    pub fn now() -> Self {
        Instant::now().into()
    }

    fn diff(&self, other: Self) -> usize {
        let sec_cmp = self.0.as_secs().cmp(&other.0.as_secs());
        let nsec_cmp = self.0.subsec_nanos().cmp(&other.0.subsec_nanos());
        match (sec_cmp, nsec_cmp) {
            (Ordering::Equal, Ordering::Greater) => {
                (self.0.subsec_nanos() - other.0.subsec_nanos()) as usize
            }
            (Ordering::Greater, Ordering::Less) => {
                (self.0.as_secs() - other.0.as_secs()) as usize * 1_000_000_000
                    - (other.0.subsec_nanos() - self.0.subsec_nanos()) as usize
            }
            (Ordering::Greater, Ordering::Equal) => {
                (self.0.as_secs() - other.0.as_secs()) as usize * 1_000_000_000
            }
            (Ordering::Greater, Ordering::Greater) => {
                (self.0.as_secs() - other.0.as_secs()) as usize * 1_000_000_000
                    + (self.0.subsec_nanos() - other.0.subsec_nanos()) as usize
            }
            _ => 0,
        }
    }

    pub fn left(&self) -> usize {
        self.diff(Expiry::now())
    }

    pub fn abs_time(&self) -> timespec {
        timespec {
            tv_sec: self.0.as_secs() as i64,
            tv_nsec: self.0.subsec_nanos() as i64,
        }
    }
}

impl From<Instant> for Expiry {
    fn from(t: Instant) -> Self {
        use std::mem;
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
    pub ctx: IoContext,
    pub expiry: Expiry,
    pub op: Option<Box<Perform>>,
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
        if let Some(op) = self.ctx.as_reactor().tq.insert(self, op) {
            this.push(op, OPERATION_CANCELED)
        }
    }

    pub fn reset_expiry(&self, expiry: Expiry) {
        if let Some(op) = self.ctx.as_reactor().tq.erase(self, expiry) {
            self.ctx.do_dispatch((op, OPERATION_CANCELED))
        }
    }

    pub fn cancel(&self) {
        if let Some(op) = self.ctx.as_reactor().tq.erase(self, Expiry::zero()) {
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
        use std::ptr;
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

#[test]
fn test_expiry_diff() {
    assert_eq!(
        Expiry(Duration::new(1, 1)).diff(Expiry(Duration::new(2, 0))),
        0
    );
    assert_eq!(
        Expiry(Duration::new(1, 1)).diff(Expiry(Duration::new(2, 1))),
        0
    );
    assert_eq!(
        Expiry(Duration::new(1, 1)).diff(Expiry(Duration::new(2, 2))),
        0
    );

    assert_eq!(
        Expiry(Duration::new(1, 1)).diff(Expiry(Duration::new(1, 2))),
        0
    );
    assert_eq!(
        Expiry(Duration::new(1, 1)).diff(Expiry(Duration::new(1, 1))),
        0
    );
    assert_eq!(
        Expiry(Duration::new(1, 1)).diff(Expiry(Duration::new(1, 0))),
        1
    );

    assert_eq!(
        Expiry(Duration::new(1, 1)).diff(Expiry(Duration::new(0, 0))),
        1_000_000_001
    );
    assert_eq!(
        Expiry(Duration::new(1, 1)).diff(Expiry(Duration::new(0, 1))),
        1_000_000_000
    );
    assert_eq!(
        Expiry(Duration::new(1, 1)).diff(Expiry(Duration::new(0, 2))),
        999_999_999
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
