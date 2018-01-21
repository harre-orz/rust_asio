#![allow(unreachable_patterns)]

use prelude::*;
use ffi::*;
use core::{AsIoContext, Exec, Perform, ThreadIoContext};
use handler::{Complete, Handler, NoYield, Yield};
use ops::{AsyncSocketOp, Failure};

use std::io;
use std::marker::PhantomData;

struct AsyncAccept<P, S, F> {
    soc: *const S,
    handler: F,
    _marker: PhantomData<P>,
}

impl<P, S, F> AsyncAccept<P, S, F> {
    fn new(soc: &S, handler: F) -> Self {
        AsyncAccept {
            soc: soc,
            handler: handler,
            _marker: PhantomData,
        }
    }
}

impl<P, S, F> Complete<(P::Socket, P::Endpoint), io::Error> for AsyncAccept<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncSocketOp,
    F: Complete<(P::Socket, P::Endpoint), io::Error>,
{
    fn success(self, this: &mut ThreadIoContext, res: (P::Socket, P::Endpoint)) {
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

impl<P, S, F> Exec for AsyncAccept<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncSocketOp,
    F: Complete<(P::Socket, P::Endpoint), io::Error>,
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

impl<P, S, F> Handler<(P::Socket, P::Endpoint), io::Error> for AsyncAccept<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncSocketOp,
    F: Complete<(P::Socket, P::Endpoint), io::Error>,
{
    type Output = ();

    type Caller = Self;

    type Callee = NoYield;

    fn channel(self) -> (Self::Caller, Self::Callee) {
        (self, NoYield)
    }
}

impl<P, S, F> Perform for AsyncAccept<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncSocketOp,
    F: Complete<(P::Socket, P::Endpoint), io::Error>,
{
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError) {
        let soc = unsafe { &*self.soc };
        if err != Default::default() {
            return self.failure(this, err.into());
        }

        loop {
            println!("call accept");
            match accept(soc) {
                Ok((acc, ep)) => {
                    let pro = soc.protocol().clone();
                    let soc = unsafe { P::Socket::from_raw_fd(this.as_ctx(), acc, pro) };
                    println!("call accept");
                    return self.success(this, (soc, ep));
                }
                Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                    return soc.add_read_op(this, self, WOULD_BLOCK)
                }
                Err(INTERRUPTED) if !soc.as_ctx().stopped() => {}
                Err(err) => return self.failure(this, err.into()),
            }
        }
    }
}

unsafe impl<P, S, F> Send for AsyncAccept<P, S, F> {}

pub fn accept_timeout<P, S>(soc: &S, timeout: &Timeout) -> io::Result<(P::Socket, P::Endpoint)>
where
    P: Protocol,
    S: Socket<P> + AsIoContext,
{
    if soc.as_ctx().stopped() {
        return Err(OPERATION_CANCELED.into());
    }

    loop {
        match accept(soc) {
            Ok((acc, ep)) => {
                let pro = soc.protocol().clone();
                let acc = unsafe { P::Socket::from_raw_fd(soc.as_ctx(), acc, pro) };
                return Ok((acc, ep));
            }
            Err(TRY_AGAIN) | Err(WOULD_BLOCK) =>
                if let Err(err) = readable(soc, &timeout) {
                    return Err(err.into());
                },
            Err(INTERRUPTED) if !soc.as_ctx().stopped() => {}
            Err(err) => return Err(err.into()),
        }
    }
}

pub fn async_accept<P, S, F>(soc: &S, handler: F) -> F::Output
where
    P: Protocol,
    S: Socket<P> + AsyncSocketOp,
    F: Handler<(P::Socket, P::Endpoint), io::Error>,
{
    let (tx, rx) = handler.channel();
    if !soc.as_ctx().stopped() {
        soc.as_ctx().do_dispatch(AsyncAccept::new(soc, tx));
    } else {
        soc.as_ctx()
            .do_dispatch(Failure::new(OPERATION_CANCELED, tx));
    }
    rx.yield_return()
}

pub fn nonblocking_accept<P, S>(soc: &S) -> io::Result<(P::Socket, P::Endpoint)>
where
    P: Protocol,
    S: Socket<P> + AsIoContext,
{
    if soc.as_ctx().stopped() {
        return Err(OPERATION_CANCELED.into());
    }

    Ok(accept(soc).map(|(acc, ep)| {
        let pro = soc.protocol().clone();
        let acc = unsafe { P::Socket::from_raw_fd(soc.as_ctx(), acc, pro) };
        (acc, ep)
    })?)
}
