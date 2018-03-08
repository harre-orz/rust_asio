use super::*;
use ffi::{SystemError};
use core::{AsIoContext, IoContext, Perform, ThreadIoContext};

use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};
use libc::{self, EV_ADD, EV_DELETE, EV_ENABLE,
           EVFILT_SIGNAL, SIG_SETMASK, sigaddset, sigprocmask};

fn kevent(kev: &Kevent, sig: i32, flags: u16) -> libc::kevent {
    libc::kevent {
        ident: sig as usize,
        filter: EVFILT_SIGNAL,
        flags: flags,
        fflags: 0,
        data: 0,
        udata: kev as *const _ as *mut _,
    }
}

pub struct KqueueSignal {
    ctx: IoContext,
    fd: Kevent,
    signals: AtomicUsize,
}

impl KqueueSignal {
    pub fn new(ctx: &IoContext) -> Box<Self> {
        let soc = Box::new(KqueueSignal {
            ctx: ctx.clone(),
            fd: Kevent::signal(),
            signals: AtomicUsize::new(0),
        });
        ctx.as_reactor().register_signal(&soc.fd);
        soc
    }

    pub fn add_read_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        this.as_ctx().clone().as_reactor().add_read_op(&self.fd, this, op, err)
    }

    pub fn cancel(&self) {
        self.fd.cancel_ops(&self.ctx, OPERATION_CANCELED)
    }

    pub fn next_read_op(&self, _: &mut ThreadIoContext) {}

    pub fn add(&self, sig: Signal) -> Result<(), SystemError> {
        let old = 1 << (sig as i32 as usize);
        if self.signals.fetch_or(old, Ordering::SeqCst) & old != 0 {
            return Err(INVALID_ARGUMENT);
        }
        let mut sigmask = self.ctx.as_reactor().sigmask.lock().unwrap();
        unsafe {
            sigaddset(&mut *sigmask, sig as i32);
            sigprocmask(SIG_SETMASK, &mut *sigmask, ptr::null_mut());
        }
        self.as_ctx().as_reactor().kevent(&[kevent(&self.fd, sig as i32, EV_ADD | EV_ENABLE)]);
        Ok(())
    }

    pub fn remove(&self, sig: Signal) -> Result<(), SystemError> {
        let old = 1 << (sig as i32 as usize);
        if self.signals.fetch_and(!old, Ordering::SeqCst) & old == 0 {
            return Err(INVALID_ARGUMENT);
        }
        self.as_ctx().as_reactor().kevent(&[kevent(&self.fd, sig as i32, EV_DELETE)]);
        Ok(())
    }

    pub fn clear(&self) {
        for sig in 0..32 {
            let old = 1 << sig;
            if self.signals.fetch_and(!old, Ordering::SeqCst) & old != 0 {
                self.as_ctx().as_reactor().kevent(&[kevent(&self.fd, sig as i32, EV_DELETE)]);
            }
        }
        debug_assert_eq!(self.signals.load(Ordering::Relaxed), 0);
    }
}

unsafe impl AsIoContext for KqueueSignal {
    fn as_ctx(&self) -> &IoContext {
        if let Some(this) = ThreadIoContext::callstack(&self.ctx) {
            this.as_ctx()
        } else {
            &self.ctx
        }
    }
}

impl Drop for KqueueSignal {
    fn drop(&mut self) {
        self.clear();
        self.ctx.as_reactor().deregister_signal(&self.fd)
    }
}
