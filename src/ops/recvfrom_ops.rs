#![allow(unreachable_patterns)]

use prelude::*;
use ffi::*;
use core::{AsIoContext, Exec, Perform, ThreadIoContext};
use ops::{Complete, Handler, NoYield, Yield, AsyncReadOp};

use std::io;
use std::slice;
use std::marker::PhantomData;

pub struct AsyncRecvFrom<P, S, F> {
    soc: *const S,
    buf: *mut u8,
    len: usize,
    flags: i32,
    handler: F,
    _marker: PhantomData<P>,
}

impl<P, S, F> AsyncRecvFrom<P, S, F> {
    pub fn new(soc: &S, buf: &[u8], flags: i32, handler: F) -> Self {
        AsyncRecvFrom {
            soc: soc,
            buf: buf.as_ptr() as *mut _,
            len: buf.len(),
            flags: flags,
            handler: handler,
            _marker: PhantomData,
        }
    }
}

unsafe impl<P, S, F> Send for AsyncRecvFrom<P, S, F> {}

impl<P, S, F> Exec for AsyncRecvFrom<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncReadOp,
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
where
    P: Protocol,
    S: Socket<P> + AsyncReadOp,
    F: Complete<(usize, P::Endpoint), io::Error>,
{
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError) {
        let soc = unsafe { &*self.soc };
        if err == Default::default() {
            while !this.as_ctx().stopped() {
                let buf = unsafe { slice::from_raw_parts_mut(self.buf, self.len) };
                match recvfrom(soc, buf, self.flags) {
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

impl<P, S, F> Handler<(usize, P::Endpoint), io::Error> for AsyncRecvFrom<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncReadOp,
    F: Complete<(usize, P::Endpoint), io::Error>,
{
    type Output = ();

    type Caller = Self;

    type Callee = NoYield;

    fn channel(self) -> (Self::Caller, Self::Callee) {
        (self, NoYield)
    }
}

impl<P, S, F> Complete<(usize, P::Endpoint), io::Error> for AsyncRecvFrom<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncReadOp,
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

pub fn async_recvfrom<P, S, F>(soc: &S, buf: &[u8], flags: i32, handler: F) -> F::Output
where
    P: Protocol,
    S: Socket<P> + AsyncReadOp,
    F: Handler<(usize, P::Endpoint), io::Error>,
{
    let (tx, rx) = handler.channel();
    soc.as_ctx().do_dispatch(
        AsyncRecvFrom::new(soc, buf, flags, tx),
    );
    rx.yield_return()
}

pub fn nonblocking_recvfrom<P, S>(
    soc: &S,
    buf: &mut [u8],
    flags: i32,
) -> io::Result<(usize, P::Endpoint)>
where
    P: Protocol,
    S: Socket<P> + AsIoContext,
{
    if soc.as_ctx().stopped() {
        return Err(OPERATION_CANCELED.into());
    }

    Ok(recvfrom(soc, buf, flags)?)
}

pub fn recvfrom_timeout<P, S>(
    soc: &S,
    buf: &mut [u8],
    flags: i32,
    timeout: &Timeout,
) -> io::Result<(usize, P::Endpoint)>
where
    P: Protocol,
    S: Socket<P> + AsIoContext,
{
    if soc.as_ctx().stopped() {
        return Err(OPERATION_CANCELED.into());
    }

    loop {
        match recvfrom(soc, buf, flags) {
            Ok(len) => return Ok(len),
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                if let Err(err) = readable(soc, timeout) {
                    return Err(err.into());
                }
            }
            Err(INTERRUPTED) if !soc.as_ctx().stopped() => (),
            Err(err) => return Err(err.into()),
        }
    }
}
