use std::io;
use std::fmt;
use std::mem;
use std::ptr;
use std::iter::Iterator;
use std::marker::PhantomData;
use {IoObject, IoService, UnsafeThreadableCell, Protocol, AsSockAddr};
use super::{IpEndpoint, Resolver, ResolverIter, UnsafeResolverIter};
use ops::*;

impl<P: Protocol, S> Resolver<P, S> {
    pub fn new<T: IoObject>(io: &T) -> Resolver<P, S> {
        Resolver {
            io: io.io_service().clone(),
            socket: UnsafeThreadableCell::new(None),
            marker: PhantomData,
        }
    }

    pub fn resolve<'a, Q: ResolverQuery<'a, P>>(&self, query: Q) -> io::Result<ResolverIter<'a, P>> {
        query.iter()
    }
}

impl<P: Protocol, S> IoObject for Resolver<P, S> {
    fn io_service(&self) -> &IoService {
        &self.io
    }
}

/// A query to be passed to a resolver.
pub trait ResolverQuery<'a, P: Protocol> {
    fn iter(self) -> io::Result<ResolverIter<'a, P>>;
}

/// A query of the resolver for the passive mode.
pub struct Passive;

/// An entry produced by a resolver.
#[derive(Clone)]
pub struct ResolverEntry<'a, P: Protocol> {
    ai: &'a addrinfo,
    marker: PhantomData<P>,
}

impl<'a, P: Protocol> ResolverEntry<'a, P> {
    pub fn endpoint(&self) -> IpEndpoint<P> {
        let mut ep = IpEndpoint::default();
        unsafe {
            let src: *const u8 = mem::transmute(self.ai.ai_addr);
            let dst: *mut u8 = mem::transmute(ep.as_mut_sockaddr());
            ptr::copy(src, dst, self.ai.ai_addrlen as usize);
        }
        ep
    }

    pub fn flags(&self) -> i32 {
        self.ai.ai_flags
    }

    pub fn is_v4(&self) -> bool {
        self.ai.ai_family == AF_INET
    }

    pub fn is_v6(&self) -> bool {
        self.ai.ai_family == AF_INET6
    }
}

impl<'a, P: Protocol> Iterator for ResolverIter<'a, P> {
    type Item = ResolverEntry<'a, P>;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.ai.is_null() {
            let ai = unsafe { &mut *self.ai };
            self.ai = ai.ai_next;
            return Some(ResolverEntry {
                ai: ai,
                marker: PhantomData,
            });
        }
        None
    }
}

impl<'a, P: Protocol> Drop for ResolverIter<'a, P> {
    fn drop(&mut self) {
        if !self.base.is_null() {
            unsafe { freeaddrinfo(self.base) };
        }
    }
}


impl<P: Protocol> UnsafeResolverIter<P> {
    pub fn next<'a>(&mut self) -> Option<ResolverEntry<'a, P>> {
        while !self.ai.is_null() {
            let ai = unsafe { &mut *self.ai };
            self.ai = ai.ai_next;
            return Some(ResolverEntry {
                ai: ai,
                marker: PhantomData,
            });
        }
        None
    }
}

impl<P: Protocol> Drop for UnsafeResolverIter<P> {
    fn drop(&mut self) {
        unsafe {
            freeaddrinfo(self.base);
        }
    }
}

impl<P: Protocol> fmt::Debug for UnsafeResolverIter<P> {
    fn fmt(&self, _: &mut fmt::Formatter) -> fmt::Result {
        Ok(())
    }
}
unsafe impl<P: Protocol> Send for UnsafeResolverIter<P> {}
