#![allow(unreachable_patterns)]

use ffi::{AsRawFd, Timeout, SystemError, TRY_AGAIN, WOULD_BLOCK, INTERRUPTED, OPERATION_CANCELED,
          send, sendto, write, writable};
use core::{Protocol, Socket, AsIoContext, Exec, Perform, ThreadIoContext, Cancel, TimeoutLoc};
use handler::{Complete, Handler, NoYield, Yield, AsyncWriteOp};

use std::io;
use std::slice;
use std::marker::PhantomData;

pub trait Writer: 'static {
    type Socket: AsRawFd + AsyncWriteOp;

    type Output: Send;

    fn write_op(&self, s: &Self::Socket, buf: &[u8]) -> Result<Self::Output, SystemError>;
}

pub struct Sent<P, S> {
    flags: i32,
    _marker: PhantomData<(P, S)>,
}

impl<P, S> Sent<P, S> {
    pub fn new(flags: i32) -> Self {
        Sent {
            flags: flags,
            _marker: PhantomData,
        }
    }
}

impl<P, S> Writer for Sent<P, S>
where
    P: Protocol,
    S: Socket<P> + AsyncWriteOp,
{
    type Socket = S;

    type Output = usize;

    fn write_op(&self, s: &Self::Socket, buf: &[u8]) -> Result<Self::Output, SystemError> {
        send(s, buf, self.flags)
    }
}

pub struct SendTo<P, S>
where
    P: Protocol,
{
    flags: i32,
    ep: P::Endpoint,
    _marker: PhantomData<(P, S)>,
}

impl<P, S> SendTo<P, S>
where
    P: Protocol,
{
    pub fn new(flags: i32, ep: &P::Endpoint) -> Self {
        SendTo {
            flags: flags,
            ep: ep.clone(),
            _marker: PhantomData,
        }
    }
}

impl<P, S> Writer for SendTo<P, S>
where
    P: Protocol,
    S: Socket<P> + AsyncWriteOp,
{
    type Socket = S;

    type Output = usize;

    fn write_op(&self, s: &Self::Socket, buf: &[u8]) -> Result<Self::Output, SystemError> {
        sendto(s, buf, self.flags, &self.ep)
    }
}

pub struct Write<S> {
    _marker: PhantomData<S>,
}

impl<S> Write<S> {
    pub fn new() -> Self {
        Write { _marker: PhantomData }
    }
}

impl<S> Writer for Write<S>
where
    S: AsRawFd + AsyncWriteOp,
{
    type Socket = S;

    type Output = usize;

    fn write_op(&self, soc: &Self::Socket, buf: &[u8]) -> Result<Self::Output, SystemError> {
        write(soc, buf)
    }
}

struct AsyncWrite<F, W>
where
    W: Writer,
{
    writer: W,
    soc: *const W::Socket,
    buf: *const u8,
    len: usize,
    handler: F,
}

unsafe impl<F, W> Send for AsyncWrite<F, W>
where
    W: Writer,
{
}

impl<F, W> Handler<W::Output, io::Error> for AsyncWrite<F, W>
where
    F: Complete<W::Output, io::Error>,
    W: Writer,
{
    type Output = ();

    type Caller = Self;

    type Callee = NoYield;

    fn channel(self) -> (Self::Caller, Self::Callee) {
        (self, NoYield)
    }
}

impl<F, W> Complete<W::Output, io::Error> for AsyncWrite<F, W>
where
    F: Complete<W::Output, io::Error>,
    W: Writer,
{
    fn success(self, this: &mut ThreadIoContext, res: W::Output) {
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

impl<F, W> Perform for AsyncWrite<F, W>
where
    F: Complete<W::Output, io::Error>,
    W: Writer,
{
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError) {
        let soc = unsafe { &*self.soc };
        if err == Default::default() {
            while !this.as_ctx().stopped() {
                let buf = unsafe { slice::from_raw_parts(self.buf, self.len) };
                match self.writer.write_op(soc, buf) {
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


impl<F, W> Exec for AsyncWrite<F, W>
where
    F: Complete<W::Output, io::Error>,
    W: Writer,
{
    fn call(self, this: &mut ThreadIoContext) {
        let soc = unsafe { &*self.soc };
        soc.add_write_op(this, Box::new(self), SystemError::default())
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        let soc = unsafe { &*self.soc };
        soc.add_write_op(this, self, SystemError::default())
    }
}



pub fn async_write_op<F, W>(soc: &W::Socket, buf: &[u8], handler: F, writer: W) -> F::Output
where
    F: Handler<W::Output, io::Error>,
    W: Writer,
{
    let (tx, rx) = handler.channel();
    soc.as_ctx().do_dispatch(AsyncWrite {
        writer: writer,
        soc: soc,
        buf: buf.as_ptr(),
        len: buf.len(),
        handler: tx,
    });
    rx.yield_wait_for(soc, soc.as_timeout(TimeoutLoc::WRITE))
}

pub fn blocking_write_op<W>(
    soc: &W::Socket,
    buf: &[u8],
    timeout: &Timeout,
    writer: W,
) -> io::Result<W::Output>
where
    W: Writer,
{
    if soc.as_ctx().stopped() {
        return Err(OPERATION_CANCELED.into());
    }
    loop {
        match writer.write_op(soc, buf) {
            Ok(len) => return Ok(len),
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                if let Err(err) = writable(soc, timeout) {
                    return Err(err.into());
                }
            }
            Err(INTERRUPTED) if !soc.as_ctx().stopped() => (),
            Err(err) => return Err(err.into()),
        }
    }
}

pub fn nonblocking_write_op<W>(soc: &W::Socket, buf: &[u8], writer: W) -> io::Result<W::Output>
where
    W: Writer,
{
    if soc.as_ctx().stopped() {
        return Err(OPERATION_CANCELED.into());
    }
    Ok(writer.write_op(soc, buf)?)
}
