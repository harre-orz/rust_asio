use prelude::*;
use ffi::*;
use core::{AsIoContext, ThreadIoContext, Task, Perform, AsyncWaitOp};
use async::{Handler, Complete, NoYield};

use std::io;
use std::marker::PhantomData;

pub struct AsyncWait<W, F> {
    wait: *mut W,
    handler: F,
}

impl<W, F> AsyncWait<W, F> {
    pub fn new(wait: &W, handler: F) -> Self {
        AsyncWait {
            wait: wait as *const _ as *mut _,
            handler: handler,
        }
    }
}

unsafe impl<W, F> Send for AsyncWait<W, F> {}

impl<W, F> Task for AsyncWait<W, F>
    where W: AsyncWaitOp + 'static,
          F: Complete<(), io::Error>
{
    fn call(self, this: &mut ThreadIoContext) {
        let wait = unsafe { &mut *self.wait };
        wait.add_wait_op(this, Box::new(self), SystemError::default())
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        let wait = unsafe { &mut *self.wait };
        wait.add_wait_op(this, self, SystemError::default())
    }
}

impl<W, F> Perform for AsyncWait<W, F>
    where W: AsyncWaitOp + 'static,
          F: Complete<(), io::Error>
{
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError) {
        let wait = unsafe { &mut *self.wait };
        wait.next_wait_op(this);

        if err == SystemError::default() {
            self.success(this, ())
        } else {
            self.failure(this, err.into())
        }
    }
}

impl<W, F> Handler<(), io::Error> for AsyncWait<W, F>
    where W: AsyncWaitOp + 'static,
          F: Complete<(), io::Error>
{
    type Output = ();

    type Perform = Self;

    type Yield = NoYield;

    fn channel(self) -> (Self::Perform, Self::Yield) {
        (self, NoYield)
    }
}

impl<W, F> Complete<(), io::Error> for AsyncWait<W, F>
    where W: AsyncWaitOp + 'static,
          F: Complete<(), io::Error>,
{
    fn success(self, this: &mut ThreadIoContext, res: ()) {
        self.handler.success(this, res)
    }

    fn failure(self, this: &mut ThreadIoContext, err: io::Error) {
        self.handler.failure(this, err)
    }
}
