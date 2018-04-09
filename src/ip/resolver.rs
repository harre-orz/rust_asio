use ffi::{SockAddr, getaddrinfo, freeaddrinfo, addrinfo, sockaddr_storage, Timeout};
use core::{Protocol, AsIoContext, IoContext, Cancel, TimeoutLoc};
use handler::Handler;
use ip::{IpEndpoint, IpProtocol};

use std::io;
use std::marker::PhantomData;
use std::ffi::CString;

/// A query to be passed to a resolver.
pub trait ResolverQuery<P> {
    fn iter(self) -> io::Result<ResolverIter<P>>;
}

impl<P, N, S> ResolverQuery<P> for (P, N, S)
where
    P: Protocol,
    N: AsRef<str>,
    S: AsRef<str>,
{
    fn iter(self) -> io::Result<ResolverIter<P>> {
        ResolverIter::new(&self.0, self.1.as_ref(), self.2.as_ref(), 0)
    }
}

/// A query of the resolver for the passive mode.
pub struct Passive;

/// An iterator over the entries produced by a resolver.
pub struct ResolverIter<P> {
    ai: *mut addrinfo,
    base: *mut addrinfo,
    _marker: PhantomData<P>,
}

impl<P> ResolverIter<P>
where
    P: Protocol,
{
    pub fn new(pro: &P, host: &str, port: &str, flags: i32) -> io::Result<ResolverIter<P>> {
        let host = CString::new(host).unwrap();
        let port = CString::new(port).unwrap();
        let ai = getaddrinfo(pro, &host, &port, flags)?;
        Ok(ResolverIter {
            ai: ai,
            base: ai,
            _marker: PhantomData,
        })
    }
}

impl<P> Drop for ResolverIter<P> {
    fn drop(&mut self) {
        freeaddrinfo(self.base)
    }
}

impl<P> Iterator for ResolverIter<P>
where
    P: IpProtocol,
{
    type Item = IpEndpoint<P>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ai.is_null() {
            None
        } else {
            unsafe {
                let ep = IpEndpoint::from_ss(SockAddr::from(
                    (*self.ai).ai_addr as *const sockaddr_storage,
                    (*self.ai).ai_addrlen as u8,
                ));
                self.ai = (&*self.ai).ai_next;
                Some(ep)
            }
        }
    }
}

unsafe impl<P> Send for ResolverIter<P> {}

/// An entry produced by a resolver.
pub struct Resolver<P> {
    ctx: IoContext,
    _marker: PhantomData<P>,
}

impl<P> Resolver<P>
where
    P: IpProtocol,
{
    pub fn new(ctx: &IoContext) -> Self {
        Resolver {
            ctx: ctx.clone(),
            _marker: PhantomData,
        }
    }

    pub fn async_connect<Q, F>(&self, query: Q, handler: F) -> F::Output
    where
        Q: ResolverQuery<P>,
        F: Handler<(P::Socket, IpEndpoint<P>), io::Error>,
    {
        async_resolve(self, self.resolve(query), handler)
    }

    pub fn connect<Q>(&self, query: Q) -> io::Result<(P::Socket, IpEndpoint<P>)>
    where
        Q: ResolverQuery<P>,
    {
        resolve(self, self.resolve(query))
    }

    pub fn resolve<Q>(&self, query: Q) -> io::Result<ResolverIter<P>>
    where
        Q: ResolverQuery<P>,
    {
        query.iter()
    }
}

impl<P> Cancel for Resolver<P> {
    fn cancel(&self) {
    }

    fn as_timeout(&self, loc: TimeoutLoc) -> &Timeout {
        unreachable!()
    }
}

unsafe impl<P> AsIoContext for Resolver<P> {
    fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }
}

use self::ops::{async_resolve, resolve};
mod ops {
    use ffi::{SERVICE_NOT_FOUND, socket};
    use core::{Socket, AsIoContext, Exec, ThreadIoContext, Cancel};
    use ip::{IpProtocol, IpEndpoint, ResolverIter};
    use handler::{Handler, Complete, Yield, NoYield, Failure};

    use std::io;
    use std::marker::PhantomData;

    struct AsyncResolve<F, P, R>
    where
        P: IpProtocol,
    {
        re: *const R,
        it: ResolverIter<P>,
        handler: F,
        res: Option<Box<(P::Socket, IpEndpoint<P>)>>,
        _marker: PhantomData<(P, R)>,
    }

    unsafe impl<F, P, R> Send for AsyncResolve<F, P, R>
    where
        P: IpProtocol,
    {
    }

    impl<F, P, R> Handler<(), io::Error> for AsyncResolve<F, P, R>
    where
        F: Complete<
            (P::Socket, IpEndpoint<P>),
            io::Error,
        >,
        P: IpProtocol,
        R: Cancel + 'static,
    {
        type Output = ();

        type Caller = Self;

        type Callee = NoYield;

        fn channel(self) -> (Self::Caller, Self::Callee) {
            (self, NoYield)
        }
    }

    impl<F, P, R> Complete<(), io::Error> for AsyncResolve<F, P, R>
    where
        F: Complete<
            (P::Socket, IpEndpoint<P>),
            io::Error,
        >,
        P: IpProtocol,
        R: Cancel + 'static,
    {
        fn success(self, this: &mut ThreadIoContext, _: ()) {
            let AsyncResolve {
                re: _,
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

    impl<F, P, R> Exec for AsyncResolve<F, P, R>
    where
        F: Complete<(P::Socket, IpEndpoint<P>), io::Error>,
        P: IpProtocol,
        R: Cancel + 'static,
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
                        unsafe { P::async_connect(&(*res).0, &(*res).1, *self); }
                    }
                    Err(err) => self.failure(this, err.into()),
                }
            } else {
                self.failure(this, SERVICE_NOT_FOUND.into());
            }
        }
    }

    pub fn async_resolve<F, P, R>(
        re: &R,
        res: io::Result<ResolverIter<P>>,
        handler: F,
    ) -> F::Output
    where
        F: Handler<(P::Socket, IpEndpoint<P>), io::Error>,
        P: IpProtocol,
        R: Cancel + 'static,
    {
        let (tx, rx) = handler.channel();
        match res {
            Ok(it) => {
                re.as_ctx().do_post(AsyncResolve {
                    re: re,
                    it: it,
                    handler: tx,
                    res: None,
                    _marker: PhantomData,
                })
            }
            Err(err) => re.as_ctx().do_dispatch(Failure::new(err, tx)),
        }
        rx.yield_wait(re)
    }

    pub fn resolve<P, R>(
        re: &R,
        res: io::Result<ResolverIter<P>>,
    ) -> io::Result<(P::Socket, IpEndpoint<P>)>
    where
        P: IpProtocol,
        R: Cancel,
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
}
