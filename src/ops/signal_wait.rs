use ffi::*;
use core::{Exec, Perform, ThreadIoContext};
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

pub fn async_signal_wait<S, F>(ctx: &S, handler: F) -> F::Output
where
    S: AsyncReadOp,
    F: Handler<Signal, io::Error>,
{
    let (tx, rx) = handler.channel();
    ctx.as_ctx().do_dispatch(SignalWait::new(ctx, tx));
    rx.yield_return()
}
