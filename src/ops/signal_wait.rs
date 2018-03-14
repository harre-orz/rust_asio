use ffi::*;
use core::{Exec, Perform, ThreadIoContext, AsIoContext, IoContext};
use ops::{Complete, Handler, NoYield, Yield, AsyncReadOp};

use std::io;

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
    S: AsRawFd + AsyncReadOp,
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
    S: AsRawFd + AsyncReadOp,
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
    S: AsRawFd + AsyncReadOp,
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
    S: AsRawFd + AsyncReadOp,
    F: Complete<Signal, io::Error>,
{
    #[cfg(target_os = "macos")]
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError) {
        match err.try_signal() {
            Ok(sig) => self.success(this, sig),
            Err(err) => self.failure(this, err.into()),
        }
    }

    #[cfg(target_os = "linux")]
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError) {
        use libc;
        use std::mem;

        if err == SystemError::default() {
            let sig = unsafe { &*self.sig };
            while !this.as_ctx().stopped() {
                unsafe {
                    let mut ssi: libc::signalfd_siginfo = mem::uninitialized();
                    match libc::read(sig.as_raw_fd(), &mut ssi as *mut _ as *mut libc::c_void, mem::size_of_val(&ssi)) {
                        -1 => match SystemError::last_error() {
                            IN_PROGRESS | WOULD_BLOCK => return sig.add_read_op(this, self, WOULD_BLOCK),
                            INTERRUPTED => {},
                            err => return self.failure(this, err.into())
                        },
                        _ => return self.success(this, mem::transmute(ssi.ssi_signo)),
                    }
                }
            }
            self.failure(this, OPERATION_CANCELED.into())
        } else {
            self.failure(this, err.into())
        }
    }
}

pub fn async_signal_wait<S, F>(ctx: &S, handler: F) -> F::Output
where
    S: AsRawFd + AsyncReadOp,
    F: Handler<Signal, io::Error>,
{
    let (tx, rx) = handler.channel();
    ctx.as_ctx().do_dispatch(SignalWait::new(ctx, tx));
    rx.yield_return()
}
