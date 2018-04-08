#![allow(unreachable_patterns)]

use ffi::{AsRawFd, Timeout, SystemError, TRY_AGAIN, WOULD_BLOCK, INTERRUPTED, OPERATION_CANCELED,
          read, recv, recvfrom, readable};
use core::{Protocol, Socket, AsIoContext, Exec, Perform, ThreadIoContext};
use handler::{Complete, Handler, NoYield, Yield, AsyncReadOp};

use std::io;
use std::slice;
use std::marker::PhantomData;

pub trait Reader: 'static {
    type Socket: AsRawFd + AsyncReadOp;

    type Output: Send;

    fn read_op(&self, s: &Self::Socket, buf: &mut [u8]) -> Result<Self::Output, SystemError>;
}

pub struct Read<S> {
    _marker: PhantomData<S>,
}

impl<S> Read<S> {
    pub fn new() -> Self {
        Read { _marker: PhantomData }
    }
}

impl<S> Reader for Read<S>
where
    S: AsRawFd + AsyncReadOp,
{
    type Socket = S;

    type Output = usize;

    fn read_op(&self, s: &Self::Socket, buf: &mut [u8]) -> Result<Self::Output, SystemError> {
        read(s, buf)
    }
}

pub struct Recv<P, S> {
    flags: i32,
    _marker: PhantomData<(P, S)>,
}

impl<P, S> Recv<P, S> {
    pub fn new(flags: i32) -> Self {
        Recv {
            flags: flags,
            _marker: PhantomData,
        }
    }
}

impl<P, S> Reader for Recv<P, S>
where
    P: Protocol,
    S: Socket<P> + AsyncReadOp,
{
    type Socket = S;

    type Output = usize;

    fn read_op(&self, s: &Self::Socket, buf: &mut [u8]) -> Result<Self::Output, SystemError> {
        recv(s, buf, self.flags)
    }
}

pub struct RecvFrom<P, S> {
    flags: i32,
    _marker: PhantomData<(P, S)>,
}

impl<P, S> RecvFrom<P, S> {
    pub fn new(flags: i32) -> Self {
        RecvFrom {
            flags: flags,
            _marker: PhantomData,
        }
    }
}

impl<P, S> Reader for RecvFrom<P, S>
where
    P: Protocol,
    S: Socket<P> + AsyncReadOp,
{
    type Socket = S;

    type Output = (usize, P::Endpoint);

    fn read_op(&self, s: &Self::Socket, buf: &mut [u8]) -> Result<Self::Output, SystemError> {
        recvfrom(s, buf, self.flags)
    }
}

struct AsyncRead<F, R>
where
    R: Reader,
{
    reader: R,
    soc: *const R::Socket,
    buf: *mut u8,
    len: usize,
    handler: F,
}

unsafe impl<F, R> Send for AsyncRead<F, R>
where
    R: Reader,
{
}

impl<F, R> Handler<R::Output, io::Error> for AsyncRead<F, R>
where
    F: Complete<R::Output, io::Error>,
    R: Reader,
{
    type Output = ();

    type Caller = Self;

    type Callee = NoYield;

    fn channel(self) -> (Self::Caller, Self::Callee) {
        (self, NoYield)
    }
}

impl<F, R> Complete<R::Output, io::Error> for AsyncRead<F, R>
where
    F: Complete<R::Output, io::Error>,
    R: Reader,
{
    fn success(self, this: &mut ThreadIoContext, res: R::Output) {
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

impl<F, R> Perform for AsyncRead<F, R>
where
    F: Complete<R::Output, io::Error>,
    R: Reader,
{
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError) {
        let soc = unsafe { &*self.soc };
        if err == Default::default() {
            while !this.as_ctx().stopped() {
                let buf = unsafe { slice::from_raw_parts_mut(self.buf, self.len) };
                match self.reader.read_op(soc, buf) {
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

impl<F, R> Exec for AsyncRead<F, R>
where
    F: Complete<R::Output, io::Error>,
    R: Reader,
{
    fn call(self, this: &mut ThreadIoContext) {
        let soc = unsafe { &*self.soc };
        soc.add_read_op(this, Box::new(self), SystemError::default())
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        let soc = unsafe { &*self.soc };
        soc.add_read_op(this, self, SystemError::default())
    }
}

pub fn async_read_op<F, R>(soc: &R::Socket, buf: &[u8], handler: F, reader: R) -> F::Output
where
    F: Handler<R::Output, io::Error>,
    R: Reader,
{
    let (tx, rx) = handler.channel();
    soc.as_ctx().do_dispatch(AsyncRead {
        reader: reader,
        soc: soc as *const R::Socket,
        buf: buf.as_ptr() as *mut u8,
        len: buf.len(),
        handler: tx,
    });
    rx.yield_return()
}

pub fn blocking_read_op<R>(
    soc: &R::Socket,
    buf: &mut [u8],
    timeout: &Timeout,
    reader: R,
) -> io::Result<R::Output>
where
    R: Reader,
{
    if soc.as_ctx().stopped() {
        return Err(OPERATION_CANCELED.into());
    }
    loop {
        match reader.read_op(soc, buf) {
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

pub fn nonblocking_read_op<R>(soc: &R::Socket, buf: &mut [u8], reader: R) -> io::Result<R::Output>
where
    R: Reader,
{
    if soc.as_ctx().stopped() {
        return Err(OPERATION_CANCELED.into());
    }
    Ok(reader.read_op(soc, buf)?)
}
