use ffi::{SystemError, OPERATION_CANCELED};
use reactor::Reactor;
use core::{AsIoContext, IoContext, Perform, ThreadIoContext};

use std::cmp::Ordering;
use std::ops::{Deref, DerefMut};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

use libc::timespec;

#[cfg(not(target_os = "linux"))]
mod nolinux;
#[cfg(not(target_os = "linux"))]
use self::nolinux::TimerCtl;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use self::linux::TimerFd as TimerCtl;

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
                (self.0.as_secs() - other.0.as_secs()) as usize * 1_000_000_000 -
                    (other.0.subsec_nanos() - self.0.subsec_nanos()) as usize
            }
            (Ordering::Greater, Ordering::Equal) => {
                (self.0.as_secs() - other.0.as_secs()) as usize * 1_000_000_000
            }
            (Ordering::Greater, Ordering::Greater) => {
                (self.0.as_secs() - other.0.as_secs()) as usize * 1_000_000_000 +
                    (self.0.subsec_nanos() - other.0.subsec_nanos()) as usize
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

#[derive(Clone)]
struct TimerImplRef(*const TimerImpl);

impl Deref for TimerImplRef {
    type Target = TimerImpl;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

impl DerefMut for TimerImplRef {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self.0 as *mut TimerImpl) }
    }
}

impl PartialEq for TimerImplRef {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl Eq for TimerImplRef {}

impl PartialOrd for TimerImplRef {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match unsafe { &*self.0 }.expiry.partial_cmp(&unsafe { &*other.0 }.expiry) {
            Some(Ordering::Equal) => self.0.partial_cmp(&other.0),
            cmp => cmp,
        }
    }
}

impl Ord for TimerImplRef {
    fn cmp(&self, other: &Self) -> Ordering {
        match unsafe { &*self.0 }.expiry.cmp(&unsafe { &*other.0 }.expiry) {
            Ordering::Equal => self.0.cmp(&other.0),
            cmp => cmp,
        }
    }
}

pub struct TimerQueue {
    mutex: Mutex<Vec<TimerImplRef>>,
    ctl: TimerCtl,
}

impl TimerQueue {
    pub fn new() -> Result<Self, SystemError> {
        Ok(TimerQueue {
            mutex: Mutex::default(),
            ctl: try!(TimerCtl::new()),
        })
    }

    pub fn startup(&self, reactor: &Reactor) {
        self.ctl.startup(reactor)
    }

    pub fn cleanup(&self, reactor: &Reactor) {
        self.ctl.cleanup(reactor)
    }

    pub fn wait_duration(&self, max: usize) -> usize {
        self.ctl.wait_duration(max)
    }

    pub fn get_ready_timers(&self, this: &mut ThreadIoContext) {
        let mut tq = self.mutex.lock().unwrap();
        let i = match tq.binary_search_by(|e| e.expiry.cmp(&Expiry::now())) {
            Ok(i) => i + 1,
            Err(i) => i,
        };
        for mut e in tq.drain(..i) {
            this.push(e.op.take().unwrap(), SystemError::default());
        }
    }

    pub fn insert(&self, timer: &TimerImpl, op: Box<Perform>) -> Option<Box<Perform>> {
        let mut tq = self.mutex.lock().unwrap();
        let mut timer = TimerImplRef(timer);
        let old_op = timer.op.take();
        timer.op = Some(op);
        let i = tq.binary_search(&timer).unwrap_err();
        tq.insert(i, timer.clone());
        if i == 0 {
            self.ctl.reset_timeout(&timer);
        }
        old_op
    }

    pub fn erase(&self, timer: &TimerImpl, expiry: Expiry) -> Option<Box<Perform>> {
        let mut tq = self.mutex.lock().unwrap();
        let mut timer = TimerImplRef(timer);
        let old_op = timer.op.take();
        if let Ok(i) = tq.binary_search(&timer) {
            tq.remove(i);
            for timer in tq.first().iter() {
                self.ctl.reset_timeout(&timer);
            }
        }
        timer.expiry = expiry;
        old_op
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

    assert!(TimerImplRef(&t1) == TimerImplRef(&t1));
    assert!(TimerImplRef(&t1) != TimerImplRef(&t2));
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

    assert!(TimerImplRef(&t1) < TimerImplRef(&t2));

    if (&t2 as *const _) < (&t3 as *const _) {
        assert!(TimerImplRef(&t2) < TimerImplRef(&t3));
    } else {
        assert!(TimerImplRef(&t3) < TimerImplRef(&t2));
    }
}
