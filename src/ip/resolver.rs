use prelude::*;
use ffi::*;
use core::{AsIoContext, IoContext};
use handler::Handler;
use ip::{IpEndpoint, IpProtocol};
use ops::*;

use std::io;
use std::mem;
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
                let ep = IpEndpoint {
                    ss: mem::transmute_copy(
                        &*((&*self.ai).ai_addr as *const SockAddr<sockaddr_storage>),
                    ),
                    _marker: PhantomData,
                };
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

unsafe impl<P> AsIoContext for Resolver<P> {
    fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }
}
