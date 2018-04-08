use ffi::{Signal, SystemError, INVALID_ARGUMENT, OPERATION_CANCELED};
use core::{AsIoContext, IoContext, Perform, ThreadIoContext, Handle, Exec};
use handler::{Handler, Complete, Yield, NoYield, AsyncReadOp};

use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};

impl Signal {
    pub fn all() -> &'static [Signal] {
        use ffi::Signal::*;
        &[
            SIGHUP,
            SIGINT,
            SIGQUIT,
            SIGILL,
            SIGABRT,
            SIGFPE,
            SIGSEGV,
            SIGPIPE,
            SIGALRM,
            SIGTERM,
            SIGUSR1,
            SIGUSR2,
            SIGCHLD,
            SIGCONT,
            SIGSTOP,
            SIGTSTP,
            SIGTTIN,
            SIGTTOU,
            SIGBUS,
            SIGPROF,
            SIGSYS,
            SIGTRAP,
            SIGURG,
            SIGVTALRM,
            SIGXCPU,
            SIGXFSZ,
        ]
    }
}

struct SignalWait<S, F> {
    sig: *const S,
    handler: F,
}

impl<S, F> SignalWait<S, F> {
    pub fn new(sig: &S, handler: F) -> Self {
        SignalWait {
            sig: sig,
            handler: handler,
        }
    }
}

unsafe impl<S, F> Send for SignalWait<S, F> {}

impl<S, F> Exec for SignalWait<S, F>
where
    S: AsyncReadOp,
    F: Complete<Signal, io::Error>,
{
    fn call(self, this: &mut ThreadIoContext) {
        let sig = unsafe { &*self.sig };
        sig.add_read_op(this, Box::new(self), SystemError::default())
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        let sig = unsafe { &*self.sig };
        sig.add_read_op(this, self, SystemError::default())
    }
}

impl<S, F> Complete<Signal, io::Error> for SignalWait<S, F>
where
    S: AsyncReadOp,
    F: Complete<Signal, io::Error>,
{
    fn success(self, this: &mut ThreadIoContext, res: Signal) {
        let sig = unsafe { &*self.sig };
        sig.next_read_op(this);
        self.handler.success(this, res)
    }

    fn failure(self, this: &mut ThreadIoContext, err: io::Error) {
        let sig = unsafe { &*self.sig };
        sig.next_read_op(this);
        self.handler.failure(this, err)
    }
}

impl<S, F> Handler<Signal, io::Error> for SignalWait<S, F>
where
    S: AsyncReadOp,
    F: Complete<Signal, io::Error>,
{
    type Output = ();

    type Caller = Self;

    type Callee = NoYield;

    fn channel(self) -> (Self::Caller, Self::Callee) {
        (self, NoYield)
    }
}

impl<S, F> Perform for SignalWait<S, F>
where
    S: AsyncReadOp,
    F: Complete<Signal, io::Error>,
{
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError) {
        match err.try_signal() {
            Ok(sig) => self.success(this, sig),
            Err(err) => self.failure(this, err.into()),
        }
    }
}

pub fn async_wait<S, F>(ctx: &S, handler: F) -> F::Output
where
    S: AsyncReadOp,
    F: Handler<Signal, io::Error>,
{
    let (tx, rx) = handler.channel();
    ctx.as_ctx().do_dispatch(SignalWait::new(ctx, tx));
    rx.yield_wait(ctx)
}

pub struct SignalImpl {
    signals: AtomicUsize,
    fd: Handle,
    ctx: IoContext,
}

impl SignalImpl {
    pub fn signal(ctx: &IoContext) -> Result<Box<Self>, SystemError> {
        let soc = Box::new(SignalImpl {
            signals: AtomicUsize::new(0),
            ctx: ctx.clone(),
            fd: Handle::signal(),
        });
        ctx.as_reactor().register_signal(&soc.fd);
        Ok(soc)
    }

    pub fn add_read_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.ctx.as_reactor().add_read_op(&self.fd, this, op, err)
    }

    pub fn next_read_op(&self, _: &mut ThreadIoContext) {}

    pub fn cancel(&self) {
        self.ctx.as_reactor().cancel_ops(
            &self.fd,
            &self.ctx,
            OPERATION_CANCELED,
        )
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
        self.ctx.as_reactor().deregister_signal(&self.fd);
    }
}
