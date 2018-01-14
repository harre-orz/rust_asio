#![allow(unreachable_patterns)]

use prelude::*;
use ffi::*;
use core::{AsIoContext, IoContext, Perform, Exec, ThreadIoContext};
use handler::{Complete, Handler, NoYield};
use ops::AsyncSocketOp;
use ip::{IpProtocol, ResolverIter};

use std::io;
use std::marker::PhantomData;

pub struct AsyncConnect<P: Protocol, S, F> {
    soc: *mut S,
    ep: P::Endpoint,
    handler: F,
    _marker: PhantomData<P>,
}

impl<P, S, F> AsyncConnect<P, S, F>
where
    P: Protocol,
{
    pub fn new(soc: &S, ep: P::Endpoint, handler: F) -> Self {
        AsyncConnect {
            soc: soc as *const _ as *mut _,
            ep: ep,
            handler: handler,
            _marker: PhantomData,
        }
    }
}

unsafe impl<P, S, F> Send for AsyncConnect<P, S, F>
where
    P: Protocol,
{
}

impl<P, S, F> Exec for AsyncConnect<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncSocketOp,
    F: Complete<(), io::Error>,
{
    fn call(self, this: &mut ThreadIoContext) {
        let soc = unsafe { &mut *self.soc };
        while !this.as_ctx().stopped() {
            match connect(soc, &self.ep) {
                Ok(()) => return self.success(this, ()),
                Err(INTERRUPTED) => (),
                Err(IN_PROGRESS) | Err(WOULD_BLOCK) => {
                    return soc.add_read_op(this, Box::new(self), IN_PROGRESS)
                }
                Err(err) => return self.failure(this, err.into()),
            }
        }
        self.failure(this, OPERATION_CANCELED.into())
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        let soc = unsafe { &mut *self.soc };
        while !this.as_ctx().stopped() {
            match connect(soc, &self.ep) {
                Ok(()) => return self.success(this, ()),
                Err(INTERRUPTED) => (),
                Err(IN_PROGRESS) | Err(WOULD_BLOCK) => {
                    return soc.add_read_op(this, self, IN_PROGRESS)
                }
                Err(err) => return self.failure(this, err.into()),
            }
        }
        self.failure(this, OPERATION_CANCELED.into())
    }
}

impl<P, S, F> Perform for AsyncConnect<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncSocketOp,
    F: Complete<(), io::Error>,
{
    fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError) {
        if err == Default::default() {
            self.success(this, ())
        } else {
            self.failure(this, err.into())
        }
    }
}

impl<P, S, F> Handler<(), io::Error> for AsyncConnect<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncSocketOp,
    F: Complete<(), io::Error>,
{
    type Output = ();

    type Perform = Self;

    type Yield = NoYield;

    fn channel(self) -> (Self::Perform, Self::Yield) {
        (self, NoYield)
    }
}

impl<P, S, F> Complete<(), io::Error> for AsyncConnect<P, S, F>
where
    P: Protocol,
    S: Socket<P> + AsyncSocketOp,
    F: Complete<(), io::Error>,
{
    fn success(self, this: &mut ThreadIoContext, res: ()) {
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


pub struct AsyncConnectIter<P: IpProtocol, F> {
    ctx: IoContext,
    it: ResolverIter<P>,
    handler: F,
    res: Option<(P::Socket, P::Endpoint)>,
    _marker: PhantomData<P>,
}

unsafe impl<P, F> Send for AsyncConnectIter<P, F>
    where P: IpProtocol {}

impl<P, F> AsyncConnectIter<P, F>
    where P: IpProtocol,
{
    pub fn new(ctx: &IoContext, it: ResolverIter<P>, handler: F) -> Self {
        AsyncConnectIter {
            ctx: ctx.clone(),
            it: it,
            handler: handler,
            res: None,
            _marker: PhantomData,
        }
    }
}

impl<P, F> Exec for AsyncConnectIter<P, F>
    where P: IpProtocol,
          F: Complete<(P::Socket, P::Endpoint), io::Error>
{
    fn call(mut self, this: &mut ThreadIoContext) {
        if let Some(ep) = self.it.next() {
            let pro = ep.protocol().clone();
            match socket(&pro) {
                Ok(soc) => {
                    self.res = Some((unsafe { P::Socket::from_raw_fd(&self.ctx, soc, pro) }, ep));
                    let &(ref soc, ref ep) = unsafe { &*(self.res.as_ref().unwrap() as *const _) };
                    P::async_connect(soc, ep, self);
                },
                Err(err) =>
                    self.failure(this, err.into())
            }
        } else {
            self.failure(this, SERVICE_NOT_FOUND.into());
        }
    }

    fn call_box(mut self: Box<Self>, this: &mut ThreadIoContext) {
        if let Some(ep) = self.it.next() {
            let pro = ep.protocol().clone();
            match socket(&pro) {
                Ok(soc) => {
                    self.res = Some((unsafe { P::Socket::from_raw_fd(&self.ctx, soc, pro) }, ep));
                    let &(ref soc, ref ep) = unsafe { &*(self.res.as_ref().unwrap() as *const _) };
                    P::async_connect(soc, ep, *self);
                },
                Err(err) =>
                    self.failure(this, err.into()),
            }
        } else {
            self.failure(this, SERVICE_NOT_FOUND.into());
        }
    }
}

impl<P, F> Handler<(), io::Error> for AsyncConnectIter<P, F>
    where P: IpProtocol,
          F: Complete<(P::Socket, P::Endpoint), io::Error>
{
    type Output = ();

    type Perform = Self;

    type Yield = NoYield;

    fn channel(self) -> (Self::Perform, Self::Yield) {
        (self, NoYield)
    }
}

impl<P, F> Complete<(), io::Error> for AsyncConnectIter<P, F>
    where P: IpProtocol,
          F: Complete<(P::Socket, P::Endpoint), io::Error>
{
    fn success(self, this: &mut ThreadIoContext, _: ()) {
        let AsyncConnectIter { ctx: _, res, it:_, handler, _marker} = self;
        handler.success(this, res.unwrap())
    }

    fn failure(self, this: &mut ThreadIoContext, err: io::Error) {
        self.handler.failure(this, err)
    }
}
