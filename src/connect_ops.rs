#![allow(unreachable_patterns)]

use ffi::{connect, connection_check, writable, Timeout, SystemError, OPERATION_CANCELED,
          IN_PROGRESS, WOULD_BLOCK, INTERRUPTED};
use core::{Protocol, AsIoContext, Socket, Exec, Perform, ThreadIoContext};
use handler::{Complete, Handler, AsyncWriteOp, Failure};

use std::io;
use std::marker::PhantomData;

struct AsyncConnect<P, S, F>
where
    P: Protocol,
{
    soc: *const S,
    ep: P::Endpoint,
    handler: F,
    _marker: PhantomData<P>,
}

unsafe impl<P, S, F> Send for AsyncConnect<P, S, F>
where
    P: Protocol,
{
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

impl<P, S, F> Exec for AsyncConnect<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncWriteOp,
    F: Complete<(), io::Error>,
{
    fn call(self, this: &mut ThreadIoContext) {
        let soc = unsafe { &*self.soc };
        if this.as_ctx().stopped() {
            return self.failure(this, OPERATION_CANCELED.into());
        }

        loop {
            match connect(soc, &self.ep) {
                Ok(()) =>
                    return self.success(this, ()),
                Err(IN_PROGRESS) | Err(WOULD_BLOCK) => {
                    return soc.add_write_op(this, Box::new(self), IN_PROGRESS)
                }
                Err(INTERRUPTED) if !soc.as_ctx().stopped() => (),
                Err(err) => return self.failure(this, err.into()),
            }
        }
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        self.call(this)
    }
}

pub fn async_connect<P, S, F>(soc: &S, ep: &P::Endpoint, handler: F) -> F::Output
where
    P: Protocol,
    S: Socket<P> + AsyncWriteOp,
    F: Handler<(), io::Error>,
{
    handler.wrap(soc, |ctx, handler| if !ctx.stopped() {
        ctx.do_dispatch(AsyncConnect {
            soc: soc,
            ep: ep.clone(),
            handler: handler,
            _marker: PhantomData,
        });
    } else {
        ctx.do_dispatch(Failure::new(OPERATION_CANCELED, handler));
    })
}


pub fn blocking_connect<P, S>(soc: &S, ep: &P::Endpoint, timeout: &Timeout) -> io::Result<()>
where
    P: Protocol,
    S: Socket<P> + AsIoContext,
{
    if soc.as_ctx().stopped() {
        return Err(OPERATION_CANCELED.into());
    }
    loop {
        match connect(soc, ep) {
            Ok(_) => return Ok(()),
            Err(IN_PROGRESS) | Err(WOULD_BLOCK) => {
                writable(soc, timeout)?;
                return Ok(connection_check(soc)?);
            }
            Err(INTERRUPTED) if !soc.as_ctx().stopped() => (),
            Err(err) => return Err(err.into()),
        }
    }
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
