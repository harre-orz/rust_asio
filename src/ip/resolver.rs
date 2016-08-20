use std::io;
use std::mem;
use std::ptr;
use std::marker::PhantomData;
use {IoObject, IoService, Protocol, SockAddr};
use super::{IpProtocol, IpEndpoint};
use backbone::{AddrInfo, addrinfo, getaddrinfo};

/// A query to be passed to a resolver.
pub trait ResolverQuery<P> {
    fn iter(self) -> io::Result<ResolverIter<P>>;
}

impl<P: Protocol, H: AsRef<str>, S: AsRef<str>> ResolverQuery<P> for (P, H, S) {
    fn iter(self) -> io::Result<ResolverIter<P>> {
        ResolverIter::new(self.0, self.1.as_ref(), self.2.as_ref(), 0)
    }
}

/// A query of the resolver for the passive mode.
pub struct Passive;

/// an iterator over the entries produced by a resolver.
pub struct ResolverIter<P> {
    _base: AddrInfo,
    ai: *mut addrinfo,
    marker: PhantomData<P>,
}

impl<P: Protocol> ResolverIter<P> {
    pub fn new(pro: P, host: &str, port: &str, flags: i32) -> io::Result<ResolverIter<P>> {
        let base = try!(getaddrinfo(pro, host, port, flags));
        let ai = base.as_ptr();
        Ok(ResolverIter {
            _base: base,
            ai: ai,
            marker: PhantomData,
        })
    }

    pub fn next_with_flags(&mut self) -> Option<(IpEndpoint<P>, i32)> {
         while !self.ai.is_null() {
            let ai = unsafe { &mut *self.ai };
            let mut ep = IpEndpoint {
                len: ai.ai_addrlen as usize,
                ss: unsafe { mem::uninitialized() },
                marker: PhantomData,
            };
            let src = ai.ai_addr as *const _ as *const u8;
            let dst = ep.as_mut_sockaddr() as *mut _ as *mut u8;
            unsafe { ptr::copy(src, dst, ep.size()); }
            self.ai = ai.ai_next;
            return Some((ep, ai.ai_flags));
        }
        None
    }
}

impl<P: Protocol> Iterator for ResolverIter<P> {
    type Item = IpEndpoint<P>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((ep, _)) = self.next_with_flags() {
            Some(ep)
        } else {
            None
        }
    }
}

unsafe impl<P> Send for ResolverIter<P> {}

/// An entry produced by a resolver.
pub struct Resolver<P> {
    io: IoService,
    marker: PhantomData<P>,
}

impl<P: IpProtocol> Resolver<P> {
    pub fn new<T: IoObject>(io: &T) -> Resolver<P> {
        Resolver {
            io: io.io_service().clone(),
            marker: PhantomData,
        }
    }

    pub fn resolve<Q: ResolverQuery<P>>(&self, query: Q) -> io::Result<ResolverIter<P>> {
        query.iter()
    }
}

impl<P> IoObject for Resolver<P> {
    fn io_service(&self) -> &IoService {
        &self.io
    }
}
