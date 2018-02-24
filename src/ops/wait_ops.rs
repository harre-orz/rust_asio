use ffi::*;
use core::{Exec, Perform, ThreadIoContext};
use ops::{Complete, Handler, NoYield, Yield, AsyncWaitOp};

use std::io;

struct AsyncWait<W, F> {
    wait: *const W,
    handler: F,
}

impl<W, F> AsyncWait<W, F> {
    fn new(wait: &W, handler: F) -> Self {
        AsyncWait {
            wait: wait,
            handler: handler,
        }
    }
}

impl<W, F> Complete<(), io::Error> for AsyncWait<W, F>
where
    W: AsyncWaitOp,
    F: Complete<(), io::Error>,
{
    fn success(self, this: &mut ThreadIoContext, res: ()) {
        self.handler.success(this, res)
    }

    fn failure(self, this: &mut ThreadIoContext, err: io::Error) {
        self.handler.failure(this, err)
    }
}

impl<W, F> Exec for AsyncWait<W, F>
where
    W: AsyncWaitOp,
    F: Complete<(), io::Error>,
{
    fn call(self, this: &mut ThreadIoContext) {
        let wait = unsafe { &*self.wait };
        wait.set_wait_op(this, Box::new(self))
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        let wait = unsafe { &*self.wait };
        wait.set_wait_op(this, self)
    }
}

impl<W, F> Handler<(), io::Error> for AsyncWait<W, F>
where
    W: AsyncWaitOp,
    F: Complete<(), io::Error>,
{
    type Output = ();

    type Caller = Self;

    type Callee = NoYield;

    fn channel(self) -> (Self::Caller, Self::Callee) {
        (self, NoYield)
    }
}

impl<W, F> Perform for AsyncWait<W, F>
where
    W: AsyncWaitOp,
    F: Complete<(), io::Error>,
{
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError) {
        if err == SystemError::default() {
            self.success(this, ())
        } else {
            self.failure(this, err.into())
        }
    }
}

unsafe impl<W, F> Send for AsyncWait<W, F> {}

pub fn async_wait<W, F>(wait: &W, handler: F) -> F::Output
where
    W: AsyncWaitOp,
    F: Handler<(), io::Error>,
{
    let (tx, rx) = handler.channel();
    wait.as_ctx().do_dispatch(AsyncWait::new(wait, tx));
    rx.yield_return()
}
