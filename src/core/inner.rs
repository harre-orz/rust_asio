use ffi::{AsRawFd, RawFd, SystemError, OPERATION_CANCELED};
use core::{AsIoContext, Expiry, Fd, IoContext, Perform, ThreadIoContext};

use std::ptr;
use std::cmp::Ordering;

pub struct InnerSocket<P> {
    ctx: IoContext,
    fd: Fd,
    pro: P,
}

impl<P> InnerSocket<P> {
    pub fn new(ctx: &IoContext, fd: RawFd, pro: P) -> Box<Self> {
        let soc = Box::new(InnerSocket {
            ctx: ctx.clone(),
            fd: Fd::socket(fd),
            pro: pro,
        });
        ctx.as_reactor().register_socket(&soc.fd);
        soc
    }

    pub fn add_read_op(&mut self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.fd.add_read_op(this, op, err)
    }

    pub fn add_write_op(&mut self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.fd.add_write_op(this, op, err)
    }

    pub fn cancel(&mut self) {
        self.fd.cancel_ops(&self.ctx)
    }

    pub fn next_read_op(&mut self, this: &mut ThreadIoContext) {
        self.fd.next_read_op(this)
    }

    pub fn next_write_op(&mut self, this: &mut ThreadIoContext) {
        self.fd.next_write_op(this)
    }

    pub fn protocol(&self) -> &P {
        &self.pro
    }
}

impl<P> Drop for InnerSocket<P> {
    fn drop(&mut self) {
        self.ctx.as_reactor().deregister_socket(&self.fd)
    }
}

unsafe impl<P> AsIoContext for InnerSocket<P> {
    fn as_ctx(&self) -> &IoContext {
        if let Some(this) = ThreadIoContext::callstack(&self.ctx) {
            this.as_ctx()
        } else {
            &self.ctx
        }
    }
}

impl<P> AsRawFd for InnerSocket<P> {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}


fn insert(tq: &mut Vec<InnerTimerPtr>, timer: &InnerTimer) -> Option<Expiry> {
    if let Err(i) = tq.binary_search(&InnerTimerPtr(timer)) {
        tq.insert(i, InnerTimerPtr(timer));
        if i == 0 {
            return Some(timer.expiry.clone());
        }
    }
    None
}

fn erase(tq: &mut Vec<InnerTimerPtr>, timer: &InnerTimer) -> Option<Expiry> {
    if let Ok(i) = tq.binary_search(&InnerTimerPtr(timer)) {
        tq.remove(i);
        if i == 0 {
            return Some(timer.expiry.clone());
        }
    }
    None
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

    pub fn cancel(&mut self) {
        let (old_op, update) = {
            let mut tq = self.ctx.as_reactor().tq.lock().unwrap();
            let old_op = self.op.take();
            (old_op, erase(&mut tq, self))
        };
        if let Some(expiry) = update {
            self.ctx.as_reactor().reset_timeout(expiry)
        }
        if let Some(op) = old_op {
            self.ctx.do_dispatch((op, OPERATION_CANCELED))
        }
    }

    pub fn set_expiry(&mut self, expiry: Expiry) {
        self.expiry = expiry;
    }

    pub fn set_wait_op(&mut self, this: &mut ThreadIoContext, op: Box<Perform>) {
        let (old_op, update) = {
            let mut tq = this.as_ctx().as_reactor().tq.lock().unwrap();
            let old_op = self.op.take();
            self.op = Some(op);
            (old_op, insert(&mut tq, self))
        };
        if let Some(expiry) = update {
            this.as_ctx().as_reactor().reset_timeout(expiry)
        }
        if let Some(op) = old_op {
            this.push(op, OPERATION_CANCELED)
        }
    }

    pub fn reset_wait_op(&mut self, this: &mut ThreadIoContext) {
        let (old_op, update) = {
            let mut tq = this.as_ctx().as_reactor().tq.lock().unwrap();
            let old_op = self.op.take();
            (old_op, erase(&mut tq, self))
        };
        if let Some(expiry) = update {
            this.as_ctx().as_reactor().reset_timeout(expiry)
        }
        if let Some(op) = old_op {
            this.push(op, OPERATION_CANCELED)
        }
    }
}

impl PartialEq for InnerTimer {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self, other)
    }
}

impl Eq for InnerTimer {}

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

impl Ord for InnerTimer {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.expiry.cmp(&other.expiry) {
            Ordering::Equal => (self as *const _ as usize).cmp(&(other as *const _ as usize)),
            cmp => cmp,
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

pub struct InnerTimerPtr(pub *const InnerTimer);

impl PartialEq for InnerTimerPtr {
    fn eq(&self, other: &Self) -> bool {
        unsafe { (&*self.0).eq(&*other.0) }
    }
}

impl Eq for InnerTimerPtr {}

impl PartialOrd for InnerTimerPtr {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        unsafe { (&*self.0).partial_cmp(&*other.0) }
    }
}

impl Ord for InnerTimerPtr {
    fn cmp(&self, other: &Self) -> Ordering {
        unsafe { (&*self.0).cmp(&*other.0) }
    }
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
