use ffi::{addrinfo, getaddrinfo, freeaddrinfo, error};
use core::{IoContext, Tx, Rx};
use prelude::*;
use socket_builder::SocketBuilder;
use ip::IpProtocol;

use std::io;
use std::marker::PhantomData;

/// A query to be passed to a resolver.
pub trait ResolverQuery<P> {
    fn iter(self) -> io::Result<ResolverIter<P>>;
}

impl<P, N, S> ResolverQuery<P> for (P, N, S)
    where P: Protocol,
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

impl<P> Drop for ResolverIter<P> {
    fn drop(&mut self) {
        freeaddrinfo(self.base)
    }
}

impl<P> ResolverIter<P>
    where P: Protocol,
{
    pub fn new(pro: &P, host: &str, port: &str, flags: i32) -> io::Result<ResolverIter<P>> {
        let ai = getaddrinfo(pro, host, port, flags).map_err(error)?;
        Ok(ResolverIter {
            ai: ai,
            base: ai,
            _marker: PhantomData,
        })
    }
}

impl<P> Iterator for ResolverIter<P>
    where P: IpProtocol,
{
    type Item = P::Endpoint;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ep) = P::from_ai(self.ai) {
            self.ai = unsafe { &*self.ai }.ai_next;
            Some(ep)
        } else {
            None
        }
    }
}

unsafe impl<P> Send for ResolverIter<P> {}

/// An entry produced by a resolver.
pub struct Resolver<P, T, R> {
    ctx: IoContext,
    _marker: PhantomData<(P, T, R)>,
}

impl<P: IpProtocol, T: Tx<P>, R: Rx<P>> Resolver<P, T, R> {
    pub fn new(ctx: &IoContext) -> Resolver<P, T, R> {
        Resolver {
            ctx: ctx.clone(),
            _marker: PhantomData,
        }
    }

    pub fn connect<Q>(&self, query: Q) -> io::Result<(T, R, P::Endpoint)>
        where Q: ResolverQuery<P>,
    {
        for ep in self.resolve(query)? {
            let pro = ep.protocol();
            if let Ok((tx, rx)) = SocketBuilder::new(&self.ctx, pro)?.connect(&ep) {
                return Ok((tx, rx, ep))
            }
        }
        Err(io::Error::new(io::ErrorKind::Other, "host not found"))
    }

    pub fn resolve<Q>(&self, query: Q) -> io::Result<ResolverIter<P>>
        where Q: ResolverQuery<P>,
    {
        query.iter()
    }
}

unsafe impl<P, T, R> AsIoContext for Resolver<P, T, R> {
    fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }
}
