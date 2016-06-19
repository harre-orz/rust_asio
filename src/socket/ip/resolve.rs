use std::io;
use std::mem;
use std::ptr;
use std::marker::PhantomData;
use socket::Protocol;
use socket::ip::*;
use ops::*;

/// An entry produced by a resolver.
#[derive(Clone)]
pub struct ResolveEntry<'a, P: Protocol> {
    ai: &'a addrinfo,
    marker: PhantomData<P>,
}

impl<'a, P: Protocol> ResolveEntry<'a, P> {
    pub fn endpoint(&self) -> IpEndpoint<P> {
        unsafe {
            let mut ep: IpEndpoint<P> = mem::zeroed();
            let src: *const u8 = mem::transmute(self.ai.ai_addr);
            let dst: *mut u8 = mem::transmute(ep.as_mut_raw_sockaddr());
            ptr::copy(src, dst, self.ai.ai_addrlen as usize);
            ep
        }
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

/// An iterator over the entries produced by a resolver.
pub struct ResolveIter<'a, P: Protocol> {
    base: &'a mut addrinfo,
    ai: *mut addrinfo,
    marker: PhantomData<P>,
}

impl<'a, P: Protocol> ResolveIter<'a, P> {
    fn new(pro: P, host: &str, port: &str, flags: i32) -> io::Result<Self> {
        let base = try!(unsafe { getaddrinfo(pro, host, port, flags) });
        Ok(ResolveIter {
            base: unsafe { &mut *base },
            ai: base,
            marker: PhantomData,
        })
    }
}

impl<'a, P: Protocol> Drop for ResolveIter<'a, P> {
    fn drop(&mut self) {
        unsafe { freeaddrinfo(self.base) }
    }
}

impl<'a, P: Protocol> Iterator for ResolveIter<'a, P> {
    type Item = ResolveEntry<'a, P>;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.ai.is_null() {
            let ai = unsafe { &mut *self.ai };
            self.ai = ai.ai_next;
            return Some(ResolveEntry {
                ai: ai,
                marker: PhantomData,
            });
        }
        None
    }
}

/// A query of the resolver for the passive mode.
pub struct Passive;

impl<'a, R: Resolver> ResolveQuery<'a, R> for (Passive, u16) {
    type Iter = ResolveIter<'a, R::Protocol>;

    fn query(self, pro: R::Protocol) -> io::Result<Self::Iter> {
        let port = self.1.to_string();
        ResolveIter::new(pro, "", &port[..], AI_PASSIVE | AI_NUMERICSERV)
    }
}

impl<'a, R: Resolver> ResolveQuery<'a, R> for (IpAddrV4, u16) {
    type Iter = ResolveIter<'a, R::Protocol>;

    fn query(self, pro: R::Protocol) -> io::Result<Self::Iter> {
        let host = self.0.to_string();
        let port = self.1.to_string();
        ResolveIter::new(pro, &host[..], &port[..], AI_NUMERICHOST | AI_NUMERICSERV)
    }
}

impl<'a, R: Resolver> ResolveQuery<'a, R> for (IpAddrV6, u16) {
    type Iter = ResolveIter<'a, R::Protocol>;

    fn query(self, pro: R::Protocol) -> io::Result<Self::Iter> {
        let host = self.0.to_string();
        let port = self.1.to_string();
        ResolveIter::new(pro, &host[..], &port[..], AI_NUMERICHOST | AI_NUMERICSERV)
    }
}

impl<'a, R: Resolver> ResolveQuery<'a, R> for (IpAddr, u16) {
    type Iter = ResolveIter<'a, R::Protocol>;

    fn query(self, pro: R::Protocol) -> io::Result<Self::Iter> {
        let host = self.0.to_string();
        let port = self.1.to_string();
        ResolveIter::new(pro, &host[..], &port[..], AI_NUMERICHOST | AI_NUMERICSERV)
    }
}

impl<'a, 'b, R: Resolver> ResolveQuery<'a, R> for (&'b IpAddrV4, u16) {
    type Iter = ResolveIter<'a, R::Protocol>;

    fn query(self, pro: R::Protocol) -> io::Result<Self::Iter> {
        let host = self.0.to_string();
        let port = self.1.to_string();
        ResolveIter::new(pro, &host[..], &port[..], AI_NUMERICHOST | AI_NUMERICSERV)
    }
}

impl<'a, 'b, R: Resolver> ResolveQuery<'a, R> for (&'b IpAddrV6, u16) {
    type Iter = ResolveIter<'a, R::Protocol>;

    fn query(self, pro: R::Protocol) -> io::Result<Self::Iter> {
        let host = self.0.to_string();
        let port = self.1.to_string();
        ResolveIter::new(pro, &host[..], &port[..], AI_NUMERICHOST | AI_NUMERICSERV)
    }
}

impl<'a, 'b, R: Resolver> ResolveQuery<'a, R> for (&'b IpAddr, u16) {
    type Iter = ResolveIter<'a, R::Protocol>;

    fn query(self, pro: R::Protocol) -> io::Result<Self::Iter> {
        let host = self.0.to_string();
        let port = self.1.to_string();
        ResolveIter::new(pro, &host[..], &port[..], AI_NUMERICHOST | AI_NUMERICSERV)
    }
}

impl<'a, 'b, R: Resolver> ResolveQuery<'a, R> for (&'b str, u16) {
    type Iter = ResolveIter<'a, R::Protocol>;

    fn query(self, pro: R::Protocol) -> io::Result<Self::Iter> {
        let port = self.1.to_string();
        ResolveIter::new(pro, self.0, &port[..], 0)
    }
}

impl<'a, 'b, 'c, R: Resolver> ResolveQuery<'a, R> for (&'b str, &'c str) {
    type Iter = ResolveIter<'a, R::Protocol>;

    fn query(self, pro: R::Protocol) -> io::Result<Self::Iter> {
        ResolveIter::new(pro, self.0, self.1, 0)
    }
}
