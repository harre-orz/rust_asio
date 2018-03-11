use ffi::{SystemError, INVALID_ARGUMENT, Signal, OPERATION_CANCELED};
use core::{AsIoContext, IoContext, Perform, ThreadIoContext, Handle};

use std::sync::atomic::{AtomicUsize, Ordering};

pub struct SignalImpl {
    ctx: IoContext,
    fd: Handle,
    signals: AtomicUsize,
}

impl SignalImpl {
    pub fn new(ctx: &IoContext) -> Box<Self> {
        let soc = Box::new(SignalImpl {
            ctx: ctx.clone(),
            fd: Handle::signal(),
            signals: AtomicUsize::new(0),
        });
        ctx.as_reactor().register_signal(&soc.fd);
        soc
    }

    pub fn add_read_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        this.as_ctx().clone().as_reactor().add_read_op(
            &self.fd,
            this,
            op,
            err,
        )
    }

    pub fn next_read_op(&self, _: &mut ThreadIoContext) {}

    pub fn cancel(&self) {
        self.ctx.clone().as_reactor().cancel_ops(&self.fd, &self.ctx, OPERATION_CANCELED)
    }

    pub fn add(&self, sig: Signal) -> Result<(), SystemError> {
        let old = 1 << (sig as i32 as usize);
        if self.signals.fetch_or(old, Ordering::SeqCst) & old != 0 {
            return Err(INVALID_ARGUMENT);
        }
        self.as_ctx().as_reactor().add_signal(&self.fd, sig);
        Ok(())
    }

    pub fn remove(&self, sig: Signal) -> Result<(), SystemError> {
        let old = 1 << (sig as i32 as usize);
        if self.signals.fetch_and(!old, Ordering::SeqCst) & old == 0 {
            return Err(INVALID_ARGUMENT);
        }
        self.as_ctx().as_reactor().del_signal(&self.fd, sig);
        Ok(())
    }

    pub fn clear(&self) {
        for sig in Signal::all() {
            let old = 1 << (*sig as i32 as usize);
            if self.signals.fetch_and(!old, Ordering::SeqCst) & old != 0 {
                self.as_ctx().as_reactor().del_signal(&self.fd, *sig);
            }
        }
        debug_assert_eq!(self.signals.load(Ordering::Relaxed), 0);
    }
}

unsafe impl AsIoContext for SignalImpl {
    fn as_ctx(&self) -> &IoContext {
        if let Some(this) = ThreadIoContext::callstack(&self.ctx) {
            this.as_ctx()
        } else {
            &self.ctx
        }
    }
}

impl Drop for SignalImpl {
    fn drop(&mut self) {
        self.clear();
        self.ctx.as_reactor().deregister_signal(&self.fd)
    }
}
