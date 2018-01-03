use prelude::*;
use ffi::*;
use core::{AsIoContext, ThreadIoContext, Task, Perform, AsyncSocket};
use async::{Handler, NoYield};

use std::io;
use std::slice;
use std::marker::PhantomData;


pub struct AsyncRead<P, S, F> {
    soc: *const S,
    buf: *mut u8,
    len: usize,
    handler: F,
    _marker: PhantomData<P>,
}

impl<P, S, F> AsyncRead<P, S, F> {
    pub fn new(soc: &S, buf: &mut [u8], handler: F) -> Self {
        AsyncRead {
            soc: soc as *const _,
            buf: buf.as_mut_ptr(),
            len: buf.len(),
            handler: handler,
            _marker: PhantomData,
        }
    }
}

unsafe impl<P, S, F> Send for AsyncRead<P, S, F> {}

impl<P, S, F> Task for AsyncRead<P, S, F>
    where P: Protocol,
          S: Socket<P> + AsyncSocket,
          F: Handler<usize, io::Error>,
{
    fn call(self, this: &mut ThreadIoContext) {
        let soc = unsafe { &*self.soc };
        soc.add_read_op(this, box self, SystemError::default())
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        self.call(this)
    }
}

impl<P, S, F> Perform for AsyncRead<P, S, F>
    where P: Protocol,
          S: Socket<P> + AsyncSocket,
          F: Handler<usize, io::Error>,
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

impl<P, S, F> Handler<usize, io::Error> for AsyncRead<P, S, F>
    where P: Protocol,
          S: Socket<P> + AsyncSocket,
          F: Handler<usize, io::Error>,
{
    type Output = ();

    type Perform = Self;

    type Yield = NoYield;

    fn channel(self) -> (Self::Perform, Self::Yield) {
        (self, NoYield)
    }

    fn complete(self, this: &mut ThreadIoContext, res: Result<usize, io::Error>) {
        let soc = unsafe { &*self.soc };
        soc.next_read_op(this);
        self.handler.complete(this, res)
    }

    fn success(self: Box<Self>, this: &mut ThreadIoContext, res: usize) {
        self.complete(this, Ok(res))
    }

    fn failure(self: Box<Self>, this: &mut ThreadIoContext, err: io::Error) {
        self.complete(this, Err(err))
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
          F: Handler<usize, io::Error>,
{
    fn call(self, this: &mut ThreadIoContext) {
        let soc = unsafe { &*self.soc };
        soc.add_read_op(this, box self, SystemError::default())
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        self.call(this)
    }
}

impl<P, S, F> Perform for AsyncRecv<P, S, F>
    where P: Protocol,
          S: Socket<P> + AsyncSocket,
          F: Handler<usize, io::Error>,
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
          F: Handler<usize, io::Error>,
{
    type Output = ();

    type Perform = Self;

    type Yield = NoYield;

    fn channel(self) -> (Self::Perform, Self::Yield) {
        (self, NoYield)
    }

    fn complete(self, this: &mut ThreadIoContext, res: Result<usize, io::Error>) {
        let soc = unsafe { &*self.soc };
        //soc.next_read_op(this);
        self.handler.complete(this, res)
    }

    fn success(self: Box<Self>, this: &mut ThreadIoContext, res: usize) {
        self.complete(this, Ok(res))
    }

    fn failure(self: Box<Self>, this: &mut ThreadIoContext, err: io::Error) {
        self.complete(this, Err(err))
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
          F: Handler<(usize, P::Endpoint), io::Error>,
{
    fn call(self, this: &mut ThreadIoContext) {
        let soc = unsafe { &*self.soc };
        soc.add_read_op(this, box self, SystemError::default())
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        self.call(this)
    }
}

impl<P, S, F> Perform for AsyncRecvFrom<P, S, F>
    where P: Protocol,
          S: Socket<P> + AsyncSocket,
          F: Handler<(usize, P::Endpoint), io::Error>,
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
          F: Handler<(usize, P::Endpoint), io::Error>,
{
    type Output = ();

    type Perform = Self;

    type Yield = NoYield;

    fn channel(self) -> (Self::Perform, Self::Yield) {
        (self, NoYield)
    }

    fn complete(self, this: &mut ThreadIoContext, res: Result<(usize, P::Endpoint), io::Error>) {
        let soc = unsafe { &*self.soc };
        //soc.next_read_op(this);
        self.handler.complete(this, res)
    }

    fn success(self: Box<Self>, this: &mut ThreadIoContext, res: (usize, P::Endpoint)) {
        self.complete(this, Ok(res))
    }

    fn failure(self: Box<Self>, this: &mut ThreadIoContext, err: io::Error) {
        self.complete(this, Err(err))
    }
}
