#![allow(unreachable_patterns)]

use prelude::*;
use ffi::*;
use core::{AsIoContext, Perform, Exec, ThreadIoContext};
use handler::{Complete, Handler, NoYield};
use ops::AsyncSocketOp;

use std::io;
use std::marker::PhantomData;

pub struct AsyncAccept<P, S, R, F> {
    soc: *mut S,
    handler: F,
    _marker: PhantomData<(P, R)>,
}

impl<P, S, R, F> AsyncAccept<P, S, R, F> {
    pub fn new(soc: &S, handler: F) -> Self {
        AsyncAccept {
            soc: soc as *const _ as *mut _,
            handler: handler,
            _marker: PhantomData,
        }
    }
}

unsafe impl<P, S, R, F> Send for AsyncAccept<P, S, R, F> {}

impl<P, S, R, F> Exec for AsyncAccept<P, S, R, F>
where
    P: Protocol,
    S: Socket<P> + AsyncSocketOp,
    R: Socket<P>,
    F: Complete<(R, P::Endpoint), io::Error>,
{
    fn call(self, this: &mut ThreadIoContext) {
        let soc = unsafe { &mut *self.soc };
        soc.add_read_op(this, Box::new(self), SystemError::default())
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        let soc = unsafe { &mut *self.soc };
        soc.add_read_op(this, self, SystemError::default())
    }
}

impl<P, S, R, F> Perform for AsyncAccept<P, S, R, F>
where
    P: Protocol,
    S: Socket<P> + AsyncSocketOp,
    R: Socket<P>,
    F: Complete<(R, P::Endpoint), io::Error>,
{
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError) {
        let soc = unsafe { &mut *self.soc };
        if err == Default::default() {
            while !this.as_ctx().stopped() {
                match accept(soc) {
                    Ok((acc, ep)) => {
                        let pro = soc.protocol().clone();
                        let soc = unsafe { R::from_raw_fd(this.as_ctx(), acc, pro) };
                        return self.success(this, (soc, ep));
                    }
                    Err(INTERRUPTED) => (),
                    Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                        return soc.add_read_op(this, self, WOULD_BLOCK)
                    }
                    Err(err) => return self.failure(this, err.into()),
                }
            }
            self.failure(this, OPERATION_CANCELED.into())
        } else {
            self.failure(this, err.into())
        }
    }
}

impl<P, S, R, F> Handler<(R, P::Endpoint), io::Error> for AsyncAccept<P, S, R, F>
where
    P: Protocol,
    S: Socket<P> + AsyncSocketOp,
    R: Socket<P>,
    F: Complete<(R, P::Endpoint), io::Error>,
{
    type Output = ();

    type Perform = Self;

    type Yield = NoYield;

    fn channel(self) -> (Self::Perform, Self::Yield) {
        (self, NoYield)
    }
}

impl<P, S, R, F> Complete<(R, P::Endpoint), io::Error> for AsyncAccept<P, S, R, F>
where
    P: Protocol,
    S: Socket<P> + AsyncSocketOp,
    R: Socket<P>,
    F: Complete<(R, P::Endpoint), io::Error>,
{
    fn success(self, this: &mut ThreadIoContext, res: (R, P::Endpoint)) {
        let soc = unsafe { &mut *self.soc };
        soc.next_read_op(this);
        self.handler.success(this, res)
    }

    fn failure(self, this: &mut ThreadIoContext, err: io::Error) {
        let soc = unsafe { &mut *self.soc };
        soc.next_read_op(this);
        self.handler.failure(this, err)
    }
}
