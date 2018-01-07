#![allow(unreachable_patterns)]

use prelude::*;
use ffi::*;
use core::{AsIoContext, ThreadIoContext, Task, Perform, AsyncSocket};
use async::{Handler, Complete, NoYield};

use std::io;
use std::slice;
use std::marker::PhantomData;


pub struct AsyncRead<S, F> {
    soc: *const S,
    buf: *mut u8,
    len: usize,
    handler: F,
}

impl<S, F> AsyncRead<S, F> {
    pub fn new(soc: &S, buf: &mut [u8], handler: F) -> Self {
        AsyncRead {
            soc: soc as *const _,
            buf: buf.as_mut_ptr(),
            len: buf.len(),
            handler: handler,
        }
    }
}

unsafe impl<S, F> Send for AsyncRead<S, F> {}

impl<S, F> Task for AsyncRead<S, F>
    where S: AsRawFd + AsyncSocket + 'static,
          F: Complete<usize, io::Error>,
{
    fn call(self, this: &mut ThreadIoContext) {
        if self.len == 0 {
            self.success(this, 0)
        } else {
            let soc = unsafe { &*self.soc };
            soc.add_read_op(this, Box::new(self), SystemError::default())
        }
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        if self.len == 0 {
            self.success(this, 0)
        } else {
            let soc = unsafe { &*self.soc };
            soc.add_read_op(this, self, SystemError::default())
        }
    }
}

impl<S, F> Perform for AsyncRead<S, F>
    where S: AsRawFd + AsyncSocket + 'static,
          F: Complete<usize, io::Error>,
{
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError) {
        let soc = unsafe { &*self.soc };
        if err == Default::default() {
            while !this.as_ctx().stopped() {
                let buf = unsafe { slice::from_raw_parts_mut(self.buf, self.len) };
                match read(soc, buf) {
                    Ok(res) =>
                        return self.success(this, res),
                    Err(INTERRUPTED) =>
                        (),
                    Err(TRY_AGAIN) | Err(WOULD_BLOCK) =>
                        return soc.add_read_op(this, self, WOULD_BLOCK),
                    Err(err) =>
                        return self.failure(this, err.into()),
                }
            }
            self.failure(this, OPERATION_CANCELED.into())
        } else {
            self.failure(this, err.into())
        }
    }
}

impl<S, F> Handler<usize, io::Error> for AsyncRead<S, F>
    where S: AsRawFd + AsyncSocket + 'static,
          F: Complete<usize, io::Error>,
{
    type Output = ();

    type Perform = Self;

    type Yield = NoYield;

    fn channel(self) -> (Self::Perform, Self::Yield) {
        (self, NoYield)
    }
}

impl<S, F> Complete<usize, io::Error> for AsyncRead<S, F>
    where S: AsRawFd + AsyncSocket + 'static,
          F: Complete<usize, io::Error>,
{
    fn success(self, this: &mut ThreadIoContext, res: usize) {
        let soc = unsafe { &*self.soc };
        soc.next_read_op(this);
        self.handler.success(this, res)
    }

    fn failure(self, this: &mut ThreadIoContext, err: io::Error) {
        let soc = unsafe { &*self.soc };
        soc.next_read_op(this);
        self.handler.failure(this, err)
    }
}



pub struct AsyncRecv<P, S, F> {
    soc: *const S,
    buf: *mut u8,
    len: usize,
    flags: i32,
    handler: F,
    _marker: PhantomData<P>,
}

impl<P, S, F> AsyncRecv<P, S, F> {
    pub fn new(soc: &S, buf: &mut [u8], flags: i32, handler: F) -> Self {
        AsyncRecv {
            soc: soc as *const _,
            buf: buf.as_mut_ptr(),
            len: buf.len(),
            flags: flags,
            handler: handler,
            _marker: PhantomData,
        }
    }
}

unsafe impl<P, S, F> Send for AsyncRecv<P, S, F> {}

impl<P, S, F> Task for AsyncRecv<P, S, F>
    where P: Protocol,
          S: Socket<P> + AsyncSocket,
          F: Complete<usize, io::Error>,
{
    fn call(self, this: &mut ThreadIoContext) {
        if self.len == 0 {
            self.success(this, 0)
        } else {
            let soc = unsafe { &*self.soc };
            soc.add_read_op(this, Box::new(self), SystemError::default())
        }
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        if self.len == 0 {
            self.success(this, 0)
        } else {
            let soc = unsafe { &*self.soc };
            soc.add_read_op(this, self, SystemError::default())
        }
    }
}

impl<P, S, F> Perform for AsyncRecv<P, S, F>
    where P: Protocol,
          S: Socket<P> + AsyncSocket,
          F: Complete<usize, io::Error>,
{
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError) {
        let soc = unsafe { &*self.soc };
        if err == Default::default() {
            while !this.as_ctx().stopped() {
                let buf = unsafe { slice::from_raw_parts_mut(self.buf, self.len) };
                match recv(soc, buf, self.flags) {
                    Ok(res) =>
                        return self.success(this, res),
                    Err(INTERRUPTED) =>
                        (),
                    Err(TRY_AGAIN) | Err(WOULD_BLOCK) =>
                        return soc.add_read_op(this, self, WOULD_BLOCK),
                    Err(err) =>
                        return self.failure(this, err.into()),
                }
            }
            self.failure(this, OPERATION_CANCELED.into())
        } else {
            self.failure(this, err.into())
        }
    }
}

impl<P, S, F> Handler<usize, io::Error> for AsyncRecv<P, S, F>
    where P: Protocol,
          S: Socket<P> + AsyncSocket,
          F: Complete<usize, io::Error>,
{
    type Output = ();

    type Perform = Self;

    type Yield = NoYield;

    fn channel(self) -> (Self::Perform, Self::Yield) {
        (self, NoYield)
    }
}

impl<P, S, F> Complete<usize, io::Error> for AsyncRecv<P, S, F>
    where P: Protocol,
          S: Socket<P> + AsyncSocket,
          F: Complete<usize, io::Error>,
{
    fn success(self, this: &mut ThreadIoContext, res: usize) {
        let soc = unsafe { &*self.soc };
        soc.next_read_op(this);
        self.handler.success(this, res)
    }

    fn failure(self, this: &mut ThreadIoContext, err: io::Error) {
        let soc = unsafe { &*self.soc };
        soc.next_read_op(this);
        self.handler.failure(this, err)
    }
}


pub struct AsyncRecvFrom<P, S, F> {
    soc: *const S,
    buf: *mut u8,
    len: usize,
    flags: i32,
    handler: F,
    _marker: PhantomData<P>,
}

impl<P, S, F> AsyncRecvFrom<P, S, F> {
    pub fn new(soc: &S, buf: &mut [u8], flags: i32, handler: F) -> Self {
        AsyncRecvFrom {
            soc: soc as *const _,
            buf: buf.as_mut_ptr(),
            len: buf.len(),
            flags: flags,
            handler: handler,
            _marker: PhantomData,
        }
    }
}

unsafe impl<P, S, F> Send for AsyncRecvFrom<P, S, F> {}

impl<P, S, F> Task for AsyncRecvFrom<P, S, F>
    where P: Protocol,
          S: Socket<P> + AsyncSocket,
          F: Complete<(usize, P::Endpoint), io::Error>,
{
    fn call(self, this: &mut ThreadIoContext) {
        let soc = unsafe { &*self.soc };
        if self.len == 0 {
            unsafe {
                let mut ep = soc.protocol().uninitialized();
                ep.resize(0);
                self.success(this, (0, ep))
            }
        } else {
            soc.add_read_op(this, Box::new(self), SystemError::default())
        }
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        let soc = unsafe { &*self.soc };
        if self.len == 0 {
            unsafe {
                let mut ep = soc.protocol().uninitialized();
                ep.resize(0);
                self.success(this, (0, ep))
            }
        } else {
            soc.add_read_op(this, self, SystemError::default())
        }
    }
}

impl<P, S, F> Perform for AsyncRecvFrom<P, S, F>
    where P: Protocol,
          S: Socket<P> + AsyncSocket,
          F: Complete<(usize, P::Endpoint), io::Error>,
{
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError) {
        let soc = unsafe { &*self.soc };
        if err == Default::default() {
            while !this.as_ctx().stopped() {
                let buf = unsafe { slice::from_raw_parts_mut(self.buf, self.len) };
                match recvfrom(soc, buf, self.flags) {
                    Ok(res) =>
                        return self.success(this, res),
                    Err(INTERRUPTED) =>
                        (),
                    Err(TRY_AGAIN) | Err(WOULD_BLOCK) =>
                        return soc.add_read_op(this, self, WOULD_BLOCK),
                    Err(err) =>
                        return self.failure(this, err.into()),
                }
            }
            self.failure(this, OPERATION_CANCELED.into())
        } else {
            self.failure(this, err.into())
        }
    }
}

impl<P, S, F> Handler<(usize, P::Endpoint), io::Error> for AsyncRecvFrom<P, S, F>
    where P: Protocol,
          S: Socket<P> + AsyncSocket,
          F: Complete<(usize, P::Endpoint), io::Error>,
{
    type Output = ();

    type Perform = Self;

    type Yield = NoYield;

    fn channel(self) -> (Self::Perform, Self::Yield) {
        (self, NoYield)
    }
}

impl<P, S, F> Complete<(usize, P::Endpoint), io::Error> for AsyncRecvFrom<P, S, F>
    where P: Protocol,
          S: Socket<P> + AsyncSocket,
          F: Complete<(usize, P::Endpoint), io::Error>,
{
    fn success(self, this: &mut ThreadIoContext, res: (usize, P::Endpoint)) {
        let soc = unsafe { &*self.soc };
        soc.next_read_op(this);
        self.handler.success(this, res)
    }

    fn failure(self, this: &mut ThreadIoContext, err: io::Error) {
        let soc = unsafe { &*self.soc };
        soc.next_read_op(this);
        self.handler.failure(this, err)
    }
}
