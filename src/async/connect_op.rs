#![allow(unreachable_patterns)]

use prelude::*;
use ffi::*;
use core::{AsIoContext, ThreadIoContext, Task, Perform};
use async::{Handler, Complete, NoYield, AsyncSocketOp};

use std::io;
use std::marker::PhantomData;


pub struct AsyncConnect<P: Protocol, S, F> {
    soc: *mut S,
    ep: P::Endpoint,
    handler: F,
    _marker: PhantomData<P>,
}

impl<P, S, F> AsyncConnect<P, S, F>
where
    P: Protocol,
{
    pub fn new(soc: &S, ep: P::Endpoint, handler: F) -> Self {
        AsyncConnect {
            soc: soc as *const _ as *mut _,
            ep: ep,
            handler: handler,
            _marker: PhantomData,
        }
    }
}

unsafe impl<P, S, F> Send for AsyncConnect<P, S, F>
where
    P: Protocol,
{
}

impl<P, S, F> Task for AsyncConnect<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncSocketOp,
    F: Complete<(), io::Error>,
{
    fn call(self, this: &mut ThreadIoContext) {
        let soc = unsafe { &mut *self.soc };
        while !this.as_ctx().stopped() {
            match connect(soc, &self.ep) {
                Ok(()) =>
                    return self.success(this, ()),
                Err(INTERRUPTED) => (),
                Err(IN_PROGRESS) | Err(WOULD_BLOCK) => return soc.add_read_op(this, Box::new(self), IN_PROGRESS),
                Err(err) => return self.failure(this, err.into()),
            }
        }
        self.failure(this, OPERATION_CANCELED.into())
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        let soc = unsafe { &mut *self.soc };
        while !this.as_ctx().stopped() {
            match connect(soc, &self.ep) {
                Ok(()) =>
                    return self.success(this, ()),
                Err(INTERRUPTED) => (),
                Err(IN_PROGRESS) | Err(WOULD_BLOCK) => return soc.add_read_op(this, self, IN_PROGRESS),
                Err(err) => return self.failure(this, err.into()),
            }
        }
        self.failure(this, OPERATION_CANCELED.into())
    }
}

impl<P, S, F> Perform for AsyncConnect<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncSocketOp,
    F: Complete<(), io::Error>,
{
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError) {
        if err == Default::default() {
            self.success(this, ())
        } else {
            self.failure(this, err.into())
        }
    }
}

impl<P, S, F> Handler<(), io::Error> for AsyncConnect<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncSocketOp,
    F: Complete<(), io::Error>,
{
    type Output = ();

    type Perform = Self;

    type Yield = NoYield;

    fn channel(self) -> (Self::Perform, Self::Yield) {
        (self, NoYield)
    }
}

impl<P, S, F> Complete<(), io::Error> for AsyncConnect<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncSocketOp,
    F: Complete<(), io::Error>,
{
    fn success(self, this: &mut ThreadIoContext, res: ()) {
        let soc = unsafe { &mut *self.soc };
        soc.next_write_op(this);
        self.handler.success(this, res)
    }

    fn failure(self, this: &mut ThreadIoContext, err: io::Error) {
        let soc = unsafe { &mut *self.soc };
        soc.next_write_op(this);
        self.handler.failure(this, err)
    }
}
