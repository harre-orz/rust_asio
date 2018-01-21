#![allow(unreachable_patterns)]

use prelude::*;
use ffi::*;
use core::{AsIoContext, Exec, ThreadIoContext};
use handler::{Complete, Handler, NoYield, Yield};
use ops::Failure;
use ip::{IpEndpoint, IpProtocol, ResolverIter};

use std::io;
use std::marker::PhantomData;

struct AsyncResolve<P: IpProtocol, R, F> {
    it: ResolverIter<P>,
    re: *const R,
    handler: F,
    res: Option<(P::Socket, IpEndpoint<P>)>,
    _marker: PhantomData<P>,
}

impl<P, R, F> AsyncResolve<P, R, F>
where
    P: IpProtocol,
{
    fn new(re: &R, it: ResolverIter<P>, handler: F) -> Self {
        AsyncResolve {
            it: it,
            re: re,
            handler: handler,
            res: None,
            _marker: PhantomData,
        }
    }
}

impl<P, R, F> Complete<(), io::Error> for AsyncResolve<P, R, F>
where
    P: IpProtocol,
    R: AsIoContext + Send + 'static,
    F: Complete<(P::Socket, IpEndpoint<P>), io::Error>,
{
    fn success(self, this: &mut ThreadIoContext, _: ()) {
        let AsyncResolve {
            it: _,
            re: _,
            res,
            handler,
            _marker,
        } = self;
        handler.success(this, res.unwrap())
    }

    fn failure(self, this: &mut ThreadIoContext, err: io::Error) {
        self.handler.failure(this, err)
    }
}

impl<P, R, F> Exec for AsyncResolve<P, R, F>
where
    P: IpProtocol,
    R: AsIoContext + Send + 'static,
    F: Complete<(P::Socket, IpEndpoint<P>), io::Error>,
{
    fn call(mut self, this: &mut ThreadIoContext) {
        let re = unsafe { &*self.re };

        if let Some(ep) = self.it.next() {
            let pro = ep.protocol().clone();
            match socket(&pro) {
                Ok(soc) => {
                    self.res = Some((
                        unsafe { P::Socket::from_raw_fd(&re.as_ctx(), soc, pro) },
                        ep,
                    ));
                    let &(ref soc, ref ep) = unsafe { &*(self.res.as_ref().unwrap() as *const _) };
                    P::async_connect(soc, ep, self);
                }
                Err(err) => self.failure(this, err.into()),
            }
        } else {
            self.failure(this, SERVICE_NOT_FOUND.into());
        }
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        self.call(this)
    }
}

impl<P, R, F> Handler<(), io::Error> for AsyncResolve<P, R, F>
where
    P: IpProtocol,
    R: AsIoContext + Send + 'static,
    F: Complete<(P::Socket, IpEndpoint<P>), io::Error>,
{
    type Output = ();

    type Caller = Self;

    type Callee = NoYield;

    fn channel(self) -> (Self::Caller, Self::Callee) {
        (self, NoYield)
    }
}

unsafe impl<P, R, F> Send for AsyncResolve<P, R, F>
where
    P: IpProtocol,
{
}

pub fn async_resolve<P, R, F>(re: &R, res: io::Result<ResolverIter<P>>, handler: F) -> F::Output
where
    P: IpProtocol,
    R: AsIoContext + Send + 'static,
    F: Handler<(P::Socket, IpEndpoint<P>), io::Error>,
{
    let (tx, rx) = handler.channel();
    match res {
        Ok(it) => re.as_ctx().do_dispatch(AsyncResolve::new(re, it, tx)),
        Err(err) => re.as_ctx().do_dispatch(Failure::new(err, tx)),
    }
    rx.yield_return()
}

pub fn resolve<P, R>(
    re: &R,
    res: io::Result<ResolverIter<P>>,
) -> io::Result<(P::Socket, IpEndpoint<P>)>
where
    P: IpProtocol,
    R: AsIoContext,
{
    for ep in res? {
        let pro = ep.protocol().clone();
        let soc = socket(&pro)?;
        let soc = unsafe { P::Socket::from_raw_fd(re.as_ctx(), soc, pro) };
        if let Ok(_) = P::connect(&soc, &ep) {
            return Ok((soc, ep));
        }
    }
    Err(SERVICE_NOT_FOUND.into())
}
