use prelude::{Protocol, SockAddr, Endpoint};
use ffi::{socket, addrinfo, getaddrinfo, freeaddrinfo};
use error::host_not_found;
use core::{IoContext, AsIoContext, Socket};
use async::Handler;
use ip::{IpProtocol, IpEndpoint};
use reactive_io::{AsAsyncFd, connect, async_connect_iterator};

use std::io;
use std::mem;
use std::ffi::CString;
use std::marker::PhantomData;

/// A query to be passed to a resolver.
pub trait ResolverQuery<P: Protocol> {
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

impl<P: Protocol> ResolverIter<P> {
    pub fn new(pro: &P, host: &str, port: &str, flags: i32) -> io::Result<ResolverIter<P>> {
        let host = try!(CString::new(host));
        let port = try!(CString::new(port));
        let ai = try!(getaddrinfo(pro, &host, &port, flags));
        Ok(ResolverIter {
            ai: ai,
            base: ai,
            _marker: PhantomData,
        })
    }
}

impl<P: Protocol> Iterator for ResolverIter<P> {
    type Item = IpEndpoint<P>;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.ai.is_null() {
            unsafe {
                let ai = &*self.ai;
                let mut ep = IpEndpoint {
                    ss: mem::transmute_copy(&*(ai.ai_addr as *const IpEndpoint<P>)),
                    _marker: PhantomData,
                };
                ep.resize(ai.ai_addrlen as usize);
                self.ai = ai.ai_next;
                return Some(ep);
            }
        }
        None
    }
}

unsafe impl<P> Send for ResolverIter<P> {}

/// An entry produced by a resolver.
pub struct Resolver<P, S> {
    ctx: IoContext,
    _marker: PhantomData<(P, S)>,
}

impl<P: IpProtocol, S: Socket<P> + AsAsyncFd> Resolver<P, S> {
    pub fn new(ctx: &IoContext) -> Resolver<P, S> {
        Resolver {
            ctx: ctx.clone(),
            _marker: PhantomData,
        }
    }

    pub fn async_connect<Q, F>(&self, query: Q, handler: F) -> F::Output
        where Q: ResolverQuery<P>,
              F: Handler<(S, IpEndpoint<P>), io::Error>,
    {
        match self.resolve(query) {
            Ok(it) => async_connect_iterator(&self.ctx, it, handler),
            Err(err) => handler.result(self.as_ctx(), Err(err)),
        }
    }

    pub fn connect<Q>(&self, query: Q) -> io::Result<(S, IpEndpoint<P>)>
        where Q: ResolverQuery<P>,
    {
        for ep in try!(self.resolve(query)) {
            let pro = ep.protocol();
            let soc = try!(socket(&pro));
            let soc = unsafe { S::from_raw_fd(&self.ctx, pro, soc) };
            if let Ok(_) = connect(&soc, &ep) {
                return Ok((soc, ep));
            }
        }
        Err(host_not_found())
    }

    pub fn resolve<Q>(&self, query: Q) -> io::Result<ResolverIter<P>>
        where Q: ResolverQuery<P>,
    {
        query.iter()
    }
}

unsafe impl<P, S> AsIoContext for Resolver<P, S> {
    fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }
}
