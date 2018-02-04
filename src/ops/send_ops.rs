#![allow(unreachable_patterns)]

use prelude::*;
use ffi::*;
use core::{AsIoContext, Exec, Perform, ThreadIoContext};
use handler::{Complete, Handler, NoYield, Yield};
use ops::AsyncWriteOp;

use std::io;
use std::slice;
use std::marker::PhantomData;

struct AsyncSend<P, S, F> {
    soc: *const S,
    buf: *const u8,
    len: usize,
    flags: i32,
    handler: F,
    _marker: PhantomData<P>,
}

impl<P, S, F> AsyncSend<P, S, F> {
    fn new(soc: &S, buf: &[u8], flags: i32, handler: F) -> Self {
        AsyncSend {
            soc: soc,
            buf: buf.as_ptr(),
            len: buf.len(),
            flags: flags,
            handler: handler,
            _marker: PhantomData,
        }
    }
}

impl<P, S, F> Complete<usize, io::Error> for AsyncSend<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncWriteOp,
    F: Complete<usize, io::Error>,
{
    fn success(self, this: &mut ThreadIoContext, res: usize) {
        let soc = unsafe { &*self.soc };
        soc.next_write_op(this);
        self.handler.success(this, res)
    }

    fn failure(self, this: &mut ThreadIoContext, err: io::Error) {
        let soc = unsafe { &*self.soc };
        soc.next_write_op(this);
        self.handler.failure(this, err)
    }
}

impl<P, S, F> Exec for AsyncSend<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncWriteOp,
    F: Complete<usize, io::Error>,
{
    fn call(self, this: &mut ThreadIoContext) {
        if self.len == 0 {
            self.success(this, 0)
        } else {
            let soc = unsafe { &*self.soc };
            soc.add_write_op(this, Box::new(self), SystemError::default())
        }
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        if self.len == 0 {
            self.success(this, 0)
        } else {
            let soc = unsafe { &*self.soc };
            soc.add_write_op(this, self, SystemError::default())
        }
    }
}

impl<P, S, F> Handler<usize, io::Error> for AsyncSend<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncWriteOp,
    F: Complete<usize, io::Error>,
{
    type Output = ();

    type Caller = Self;

    type Callee = NoYield;

    fn channel(self) -> (Self::Caller, Self::Callee) {
        (self, NoYield)
    }
}

impl<P, S, F> Perform for AsyncSend<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncWriteOp,
    F: Complete<usize, io::Error>,
{
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError) {
        let soc = unsafe { &*self.soc };
        if err == Default::default() {
            while !this.as_ctx().stopped() {
                let buf = unsafe { slice::from_raw_parts(self.buf, self.len) };
                match send(soc, buf, self.flags) {
                    Ok(res) => return self.success(this, res),
                    Err(INTERRUPTED) => (),
                    Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                        return soc.add_write_op(this, self, WOULD_BLOCK)
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

unsafe impl<P, S, F> Send for AsyncSend<P, S, F> {}

pub fn async_send<P, S, F>(soc: &S, buf: &[u8], flags: i32, handler: F) -> F::Output
where
    P: Protocol,
    S: Socket<P> + AsyncWriteOp,
    F: Handler<usize, io::Error>,
{
    let (tx, rx) = handler.channel();
    soc.as_ctx()
        .do_dispatch(AsyncSend::new(soc, buf, flags, tx));
    rx.yield_return()
}

pub fn nonblocking_send<P, S>(soc: &S, buf: &[u8], flags: i32) -> io::Result<usize>
where
    P: Protocol,
    S: Socket<P> + AsIoContext,
{
    if soc.as_ctx().stopped() {
        return Err(OPERATION_CANCELED.into());
    }

    Ok(send(soc, buf, flags)?)
}

pub fn send_timeout<P, S>(soc: &S, buf: &[u8], flags: i32, timeout: &Timeout) -> io::Result<usize>
where
    P: Protocol,
    S: Socket<P> + AsIoContext,
{
    if soc.as_ctx().stopped() {
        return Err(OPERATION_CANCELED.into());
    }

    loop {
        match send(soc, buf, flags) {
            Ok(len) => return Ok(len),
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => if let Err(err) = readable(soc, timeout) {
                return Err(err.into());
            },
            Err(INTERRUPTED) if !soc.as_ctx().stopped() => (),
            Err(err) => return Err(err.into()),
        }
    }
}
