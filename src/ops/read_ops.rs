#![allow(unreachable_patterns)]

use ffi::*;
use core::{AsIoContext, Exec, Perform, ThreadIoContext};
use handler::{Complete, Handler, NoYield, Yield};
use ops::AsyncReadOp;

use std::io;
use std::slice;

pub struct AsyncRead<S, F> {
    soc: *const S,
    buf: *mut u8,
    len: usize,
    handler: F,
}

impl<S, F> AsyncRead<S, F> {
    pub fn new(soc: &S, buf: &[u8], handler: F) -> Self {
        AsyncRead {
            soc: soc,
            buf: buf.as_ptr() as *mut _,
            len: buf.len(),
            handler: handler,
        }
    }
}

impl<S, F> Complete<usize, io::Error> for AsyncRead<S, F>
where
    S: AsRawFd + AsyncReadOp,
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

impl<S, F> Exec for AsyncRead<S, F>
where
    S: AsRawFd + AsyncReadOp,
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

impl<S, F> Handler<usize, io::Error> for AsyncRead<S, F>
where
    S: AsRawFd + AsyncReadOp,
    F: Complete<usize, io::Error>,
{
    type Output = ();

    type Caller = Self;

    type Callee = NoYield;

    fn channel(self) -> (Self::Caller, Self::Callee) {
        (self, NoYield)
    }
}

impl<S, F> Perform for AsyncRead<S, F>
where
    S: AsRawFd + AsyncReadOp,
    F: Complete<usize, io::Error>,
{
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError) {
        let soc = unsafe { &*self.soc };
        if err == Default::default() {
            while !this.as_ctx().stopped() {
                let buf = unsafe { slice::from_raw_parts_mut(self.buf, self.len) };
                match read(soc, buf) {
                    Ok(res) => return self.success(this, res),
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

unsafe impl<S, F> Send for AsyncRead<S, F> {}

pub fn async_read<S, F>(soc: &S, buf: &[u8], handler: F) -> F::Output
where
    S: AsRawFd + AsyncReadOp,
    F: Handler<usize, io::Error>,
{
    let (tx, rx) = handler.channel();
    soc.as_ctx().do_dispatch(AsyncRead::new(soc, buf, tx));
    rx.yield_return()
}

pub fn nonblocking_read<S>(soc: &S, buf: &mut [u8]) -> io::Result<usize>
where
    S: AsRawFd + AsIoContext,
{
    if soc.as_ctx().stopped() {
        return Err(OPERATION_CANCELED.into());
    }

    Ok(read(soc, buf)?)
}

pub fn read_timeout<S>(soc: &S, buf: &mut [u8], timeout: &Timeout) -> io::Result<usize>
where
    S: AsRawFd + AsIoContext,
{
    if soc.as_ctx().stopped() {
        return Err(OPERATION_CANCELED.into());
    }

    loop {
        match read(soc, buf) {
            Ok(len) => return Ok(len),
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => if let Err(err) = readable(soc, timeout) {
                return Err(err.into());
            },
            Err(INTERRUPTED) if !soc.as_ctx().stopped() => (),
            Err(err) => return Err(err.into()),
        }
    }
}
