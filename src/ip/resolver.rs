use std::io;
use std::ptr;
use std::marker::PhantomData;
use {IoObject, IoService, Protocol, Endpoint, FromRawFd};
use super::{IpProtocol, IpEndpoint};
use backbone::{AddrInfo, addrinfo, getaddrinfo, socket, bind, connect};

fn not_found_host() -> io::Error {
    io::Error::new(io::ErrorKind::Other, "Not found host")
}

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
}

impl<P: Protocol> Iterator for ResolverIter<P> {
    type Item = (IpEndpoint<P>, i32);

    fn next(&mut self) -> Option<Self::Item> {
        while !self.ai.is_null() {
            let mut ep = IpEndpoint::default();
            let ai = unsafe { &mut *self.ai };
            unsafe {
                let src = ai.ai_addr as *const _ as *const u8;
                let dst = ep.as_mut_sockaddr() as *mut _ as *mut u8;
                ptr::copy(src, dst, ai.ai_addrlen as usize);
                self.ai = ai.ai_next;
            }
            return Some((ep, ai.ai_flags));
        }
        None
    }
}

unsafe impl<P> Send for ResolverIter<P> {}

unsafe impl<P> Sync for ResolverIter<P> {}


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

    fn protocol(ep: &IpEndpoint<P>) -> P {
        if ep.is_v4() {
            P::v4()
        } else if ep.is_v6() {
            P::v6()
        } else {
            unreachable!();
        }
    }

    pub fn bind<Q: ResolverQuery<P>, S: FromRawFd<P>>(&self, query: Q) -> io::Result<(S, IpEndpoint<P>)> {
        for (ep, _) in try!(self.resolve(query)) {
            let pro = Self::protocol(&ep);
            let fd = try!(socket(&pro));
            let soc = unsafe { S::from_raw_fd(self, pro, fd) };
            if let Ok(_) = bind(&soc, &ep) {
                return Ok((soc, ep));
            }
        }
        Err(not_found_host())
    }

    pub fn connect<Q: ResolverQuery<P>, S: FromRawFd<P>>(&self, query: Q) -> io::Result<(S, IpEndpoint<P>)> {
        for (ep, _) in try!(self.resolve(query)) {
            let pro = Self::protocol(&ep);
            let fd = try!(socket(&pro));
            let soc = unsafe { S::from_raw_fd(self, pro, fd) };
            if let Ok(_) = connect(&soc, &ep) {
                return Ok((soc, ep));
            }
        }
        Err(not_found_host())
    }
}

impl<P> IoObject for Resolver<P> {
    fn io_service(&self) -> &IoService {
        &self.io
    }
}
