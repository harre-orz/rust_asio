#![allow(unreachable_patterns)]

use prelude::*;
use ffi::*;
use core::{AsIoContext, Exec, Perform, ThreadIoContext};
use handler::{Complete, Handler, NoYield, Yield};
use ops::{AsyncWriteOp, Failure};

use std::io;
use std::marker::PhantomData;

struct AsyncConnect<P: Protocol, S, F> {
    soc: *const S,
    ep: P::Endpoint,
    handler: F,
    _marker: PhantomData<P>,
}

impl<P, S, F> AsyncConnect<P, S, F>
where
    P: Protocol,
{
    fn new(soc: &S, ep: P::Endpoint, handler: F) -> Self {
        AsyncConnect {
            soc: soc,
            ep: ep,
            handler: handler,
            _marker: PhantomData,
        }
    }
}

impl<P, S, F> Complete<(), io::Error> for AsyncConnect<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncWriteOp,
    F: Complete<(), io::Error>,
{
    fn success(self, this: &mut ThreadIoContext, res: ()) {
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

impl<P, S, F> Exec for AsyncConnect<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncWriteOp,
    F: Complete<(), io::Error>,
{
    fn call(self, this: &mut ThreadIoContext) {
        let soc = unsafe { &*self.soc };
        if this.as_ctx().stopped() {
            return self.failure(this, OPERATION_CANCELED.into())
        }

        loop {
            let ret = connect(soc, &self.ep);
            println!("connect {:?}", &ret);
            match ret {
                Ok(()) =>
                    return self.success(this, ()),
                Err(IN_PROGRESS) | Err(WOULD_BLOCK) =>
                    return soc.add_write_op(this, Box::new(self), IN_PROGRESS),
                Err(INTERRUPTED) if !soc.as_ctx().stopped() =>
                    (),
                Err(err) =>
                    return self.failure(this, err.into()),
            }
        }
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        let soc = unsafe { &*self.soc };
        if this.as_ctx().stopped() {
            return self.failure(this, OPERATION_CANCELED.into())
        }

        loop {
            let ret = connect(soc, &self.ep);
            println!("connect {:?}", &ret);
            match ret {
                Ok(()) =>
                    return self.success(this, ()),
                Err(IN_PROGRESS) | Err(WOULD_BLOCK) =>
                    return soc.add_write_op(this, self, IN_PROGRESS),
                Err(INTERRUPTED) if !soc.as_ctx().stopped() =>
                    (),
                Err(err) =>
                    return self.failure(this, err.into()),
            }
        }
    }
}

impl<P, S, F> Handler<(), io::Error> for AsyncConnect<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncWriteOp,
    F: Complete<(), io::Error>,
{
    type Output = ();

    type Caller = Self;

    type Callee = NoYield;

    fn channel(self) -> (Self::Caller, Self::Callee) {
        (self, NoYield)
    }
}

impl<P, S, F> Perform for AsyncConnect<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncWriteOp,
    F: Complete<(), io::Error>,
{
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError) {
        let soc = unsafe { &*self.soc };
        if err == Default::default() {
            match connection_check(soc) {
                Ok(_) => self.success(this, ()),
                Err(err) => self.failure(this, err.into()),
            }
        } else {
            self.failure(this, err.into())
        }
    }
}

unsafe impl<P, S, F> Send for AsyncConnect<P, S, F>
where
    P: Protocol,
{
}

pub fn nonblocking_connect<P, S>(soc: &S, ep: &P::Endpoint) -> io::Result<()>
where
    P: Protocol,
    S: Socket<P> + AsIoContext,
{
    if soc.as_ctx().stopped() {
        return Err(OPERATION_CANCELED.into());
    }

    Ok(connect(soc, ep)?)
}

pub fn connect_timeout<P, S>(soc: &S, ep: &P::Endpoint, timeout: &Timeout) -> io::Result<()>
where
    P: Protocol,
    S: Socket<P> + AsIoContext,
{
    if soc.as_ctx().stopped() {
        return Err(OPERATION_CANCELED.into());
    }

    loop {
        match connect(soc, ep) {
            Ok(_) =>
                return Ok(()),
            Err(IN_PROGRESS) | Err(WOULD_BLOCK) => {
                writable(soc, timeout)?;
                return Ok(connection_check(soc)?)
            },
            Err(INTERRUPTED) if !soc.as_ctx().stopped() =>
                (),
            Err(err) =>
                return Err(err.into()),
        }
    }
}

pub fn async_connect<P, S, F>(soc: &S, ep: &P::Endpoint, handler: F) -> F::Output
where
    P: Protocol,
    S: Socket<P> + AsyncWriteOp,
    F: Handler<(), io::Error>,
{
    let (tx, rx) = handler.channel();
    if !soc.as_ctx().stopped() {
        soc.as_ctx().do_dispatch(AsyncConnect::new(soc, ep.clone(), tx));
    } else {
        soc.as_ctx().do_dispatch(Failure::new(OPERATION_CANCELED, tx));
    }
    rx.yield_return()
}
