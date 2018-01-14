#![allow(unreachable_patterns)]

use prelude::*;
use ffi::*;
use core::{AsIoContext, ThreadIoContext, Task, Perform};
use async::{Handler, Complete, NoYield, AsyncSocketOp};

use std::io;
use std::slice;
use std::marker::PhantomData;


pub struct AsyncSend<P, S, F> {
    soc: *mut S,
    buf: *const u8,
    len: usize,
    flags: i32,
    handler: F,
    _marker: PhantomData<P>,
}

impl<P, S, F> AsyncSend<P, S, F> {
    pub fn new(soc: &S, buf: &[u8], flags: i32, handler: F) -> Self {
        AsyncSend {
            soc: soc as *const _ as *mut _,
            buf: buf.as_ptr(),
            len: buf.len(),
            flags: flags,
            handler: handler,
            _marker: PhantomData,
        }
    }
}

unsafe impl<P, S, F> Send for AsyncSend<P, S, F> {}

impl<P, S, F> Task for AsyncSend<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncSocketOp,
    F: Complete<usize, io::Error>,
{
    fn call(self, this: &mut ThreadIoContext) {
        if self.len == 0 {
            self.success(this, 0)
        } else {
            let soc = unsafe { &mut *self.soc };
            soc.add_write_op(this, Box::new(self), SystemError::default())
        }
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        if self.len == 0 {
            self.success(this, 0)
        } else {
            let soc = unsafe { &mut *self.soc };
            soc.add_write_op(this, self, SystemError::default())
        }
    }
}

impl<P, S, F> Perform for AsyncSend<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncSocketOp,
    F: Complete<usize, io::Error>,
{
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError) {
        let soc = unsafe { &mut *self.soc };
        if err == Default::default() {
            while !this.as_ctx().stopped() {
                let buf = unsafe { slice::from_raw_parts(self.buf, self.len) };
                match send(soc, buf, self.flags) {
                    Ok(res) => return self.success(this, res),
                    Err(INTERRUPTED) => (),
                    Err(TRY_AGAIN) | Err(WOULD_BLOCK) => return soc.add_write_op(this, self, WOULD_BLOCK),
                    Err(err) => return self.failure(this, err.into()),
                }
            }
            self.failure(this, OPERATION_CANCELED.into())
        } else {
            self.failure(this, err.into())
        }
    }
}

impl<P, S, F> Handler<usize, io::Error> for AsyncSend<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncSocketOp,
    F: Complete<usize, io::Error>,
{
    type Output = ();

    type Perform = Self;

    type Yield = NoYield;

    fn channel(self) -> (Self::Perform, Self::Yield) {
        (self, NoYield)
    }
}

impl<P, S, F> Complete<usize, io::Error> for AsyncSend<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncSocketOp,
    F: Complete<usize, io::Error>,
{
    fn success(self, this: &mut ThreadIoContext, res: usize) {
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


pub struct AsyncSendTo<P: Protocol, S, F> {
    soc: *mut S,
    buf: *const u8,
    len: usize,
    ep: P::Endpoint,
    flags: i32,
    handler: F,
    _marker: PhantomData<P>,
}

impl<P, S, F> AsyncSendTo<P, S, F>
where
    P: Protocol,
{
    pub fn new(soc: &S, buf: &[u8], flags: i32, ep: P::Endpoint, handler: F) -> Self {
        AsyncSendTo {
            soc: soc as *const _ as *mut _,
            buf: buf.as_ptr(),
            len: buf.len(),
            flags: flags,
            ep: ep,
            handler: handler,
            _marker: PhantomData,
        }
    }
}

unsafe impl<P, S, F> Send for AsyncSendTo<P, S, F>
where
    P: Protocol,
{
}

impl<P, S, F> Task for AsyncSendTo<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncSocketOp,
    F: Complete<usize, io::Error>,
{
    fn call(self, this: &mut ThreadIoContext) {
        if self.len == 0 {
            self.success(this, 0)
        } else {
            let soc = unsafe { &mut *self.soc };
            soc.add_write_op(this, Box::new(self), SystemError::default())
        }
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        if self.len == 0 {
            self.success(this, 0)
        } else {
            let soc = unsafe { &mut *self.soc };
            soc.add_write_op(this, self, SystemError::default())
        }
    }
}

impl<P, S, F> Perform for AsyncSendTo<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncSocketOp,
    F: Complete<usize, io::Error>,
{
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError) {
        let soc = unsafe { &mut *self.soc };
        if err == Default::default() {
            while !this.as_ctx().stopped() {
                let buf = unsafe { slice::from_raw_parts(self.buf, self.len) };
                match sendto(soc, buf, self.flags, &self.ep) {
                    Ok(res) => return self.success(this, res),
                    Err(INTERRUPTED) => (),
                    Err(TRY_AGAIN) | Err(WOULD_BLOCK) => return soc.add_write_op(this, self, WOULD_BLOCK),
                    Err(err) => return self.failure(this, err.into()),
                }
            }
            self.failure(this, OPERATION_CANCELED.into())
        } else {
            self.failure(this, err.into())
        }
    }
}

impl<P, S, F> Handler<usize, io::Error> for AsyncSendTo<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncSocketOp,
    F: Complete<usize, io::Error>,
{
    type Output = ();

    type Perform = Self;

    type Yield = NoYield;

    fn channel(self) -> (Self::Perform, Self::Yield) {
        (self, NoYield)
    }
}

impl<P, S, F> Complete<usize, io::Error> for AsyncSendTo<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncSocketOp,
    F: Complete<usize, io::Error>,
{
    fn success(self, this: &mut ThreadIoContext, res: usize) {
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


pub struct AsyncWrite<S, F> {
    soc: *mut S,
    buf: *const u8,
    len: usize,
    handler: F,
}

impl<S, F> AsyncWrite<S, F> {
    pub fn new(soc: &S, buf: &[u8], handler: F) -> Self {
        AsyncWrite {
            soc: soc as *const _ as *mut _,
            buf: buf.as_ptr(),
            len: buf.len(),
            handler: handler,
        }
    }
}

unsafe impl<S, F> Send for AsyncWrite<S, F> {}

impl<S, F> Task for AsyncWrite<S, F>
where
    S: AsRawFd + AsyncSocketOp,
    F: Complete<usize, io::Error>,
{
    fn call(self, this: &mut ThreadIoContext) {
        if self.len == 0 {
            self.success(this, 0)
        } else {
            let soc = unsafe { &mut *self.soc };
            soc.add_write_op(this, Box::new(self), SystemError::default())
        }
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        if self.len == 0 {
            self.success(this, 0)
        } else {
            let soc = unsafe { &mut *self.soc };
            soc.add_write_op(this, self, SystemError::default())
        }
    }
}

impl<S, F> Perform for AsyncWrite<S, F>
where
    S: AsRawFd + AsyncSocketOp,
    F: Complete<usize, io::Error>,
{
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError) {
        let soc = unsafe { &mut *self.soc };
        if err == Default::default() {
            while !this.as_ctx().stopped() {
                let buf = unsafe { slice::from_raw_parts(self.buf, self.len) };
                match write(soc, buf) {
                    Ok(res) => return self.success(this, res),
                    Err(INTERRUPTED) => (),
                    Err(TRY_AGAIN) | Err(WOULD_BLOCK) => return soc.add_write_op(this, self, WOULD_BLOCK),
                    Err(err) => return self.failure(this, err.into()),
                }
            }
            self.failure(this, OPERATION_CANCELED.into())
        } else {
            self.failure(this, err.into())
        }
    }
}

impl<S, F> Handler<usize, io::Error> for AsyncWrite<S, F>
where
    S: AsRawFd + AsyncSocketOp,
    F: Complete<usize, io::Error>,
{
    type Output = ();

    type Perform = Self;

    type Yield = NoYield;

    fn channel(self) -> (Self::Perform, Self::Yield) {
        (self, NoYield)
    }
}

impl<S, F> Complete<usize, io::Error> for AsyncWrite<S, F>
where
    S: AsRawFd + AsyncSocketOp,
    F: Complete<usize, io::Error>,
{
    fn success(self, this: &mut ThreadIoContext, res: usize) {
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
