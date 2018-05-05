use ffi::{SERVICE_NOT_FOUND, Timeout, socket};
use core::{IoContext, Socket, AsIoContext, Exec, ThreadIoContext, Cancel};
use ip::{IpProtocol, IpEndpoint, ResolverIter};
use handler::{Handler, Complete, Failure};

use std::io;
use std::marker::PhantomData;

struct AsyncResolve<F, P>
where
    P: IpProtocol,
{
    it: ResolverIter<P>,
    handler: F,
    res: Option<Box<(P::Socket, IpEndpoint<P>)>>,
    _marker: PhantomData<P>,
}

unsafe impl<F, P> Send for AsyncResolve<F, P>
where
    P: IpProtocol,
{
}

impl<F, P> Handler<(), io::Error> for AsyncResolve<F, P>
where
    F: Complete<
        (P::Socket, IpEndpoint<P>),
        io::Error,
    >,
    P: IpProtocol,
{
    type Output = ();

    type WrappedHandler = Self;

    fn wrap<W>(self, ctx: &IoContext, wrapper: W) -> Self::Output
    where
        W: FnOnce(&IoContext, Self::WrappedHandler),
    {
        wrapper(ctx, self)
    }

    fn wrap_timeout<W>(self, ctx: &Cancel, _: &Timeout, wrapper: W) -> Self::Output
    where
        W: FnOnce(&IoContext, Self::WrappedHandler),
    {
        wrapper(ctx.as_ctx(), self)
    }
}

impl<F, P> Complete<(), io::Error> for AsyncResolve<F, P>
where
    F: Complete<
        (P::Socket, IpEndpoint<P>),
        io::Error,
    >,
    P: IpProtocol,
{
    fn success(self, this: &mut ThreadIoContext, _: ()) {
        let AsyncResolve {
            it: _,
            res,
            handler,
            _marker,
        } = self;
        this.decrease_outstanding_work();
        handler.success(this, *res.unwrap())
    }

    fn failure(self, this: &mut ThreadIoContext, err: io::Error) {
        this.decrease_outstanding_work();
        self.handler.failure(this, err)
    }
}

impl<F, P> Exec for AsyncResolve<F, P>
where
    F: Complete<(P::Socket, IpEndpoint<P>), io::Error>,
    P: IpProtocol,
{
    fn call(self, _: &mut ThreadIoContext) {
        unreachable!("");
    }

    fn call_box(mut self: Box<Self>, this: &mut ThreadIoContext) {
        if let Some(ep) = self.it.next() {
            let pro = ep.protocol().clone();
            match socket(&pro) {
                Ok(soc) => {
                    self.res = Some(Box::new((
                        unsafe { P::Socket::from_raw_fd(this.as_ctx(), soc, pro) },
                        ep,
                    )));
                    // FIXME
                    let res = &**self.res.as_ref().unwrap() as *const (P::Socket, IpEndpoint<P>);
                    unsafe {
                        P::async_connect(&(*res).0, &(*res).1, *self);
                    }
                }
                Err(err) => self.failure(this, err.into()),
            }
        } else {
            self.failure(this, SERVICE_NOT_FOUND.into());
        }
    }
}

pub fn async_resolve<F, P, R>(re: &R, res: io::Result<ResolverIter<P>>, handler: F) -> F::Output
where
    F: Handler<(P::Socket, IpEndpoint<P>), io::Error>,
    P: IpProtocol,
    R: Cancel + Send + 'static,
{
    handler.wrap(re.as_ctx(), |ctx, handler| match res {
        Ok(it) => {
            ctx.do_post(AsyncResolve {
                it: it,
                handler: handler,
                res: None,
                _marker: PhantomData,
            })
        }
        Err(err) => ctx.do_dispatch(Failure::new(err, handler)),
    })
}

pub fn resolve<P, R>(
    re: &R,
    res: io::Result<ResolverIter<P>>,
) -> io::Result<(P::Socket, IpEndpoint<P>)>
where
    R: Cancel,
    P: IpProtocol,
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
