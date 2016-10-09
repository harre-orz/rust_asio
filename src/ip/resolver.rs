use std::io;
use std::mem;
use std::ptr;
use std::marker::PhantomData;
use {IoObject, IoService, Protocol, SockAddr, FromRawFd};
use super::{IpProtocol, IpEndpoint};
use async_result::{Handler, NullAsyncResult};
use backbone::{AddrInfo, addrinfo, getaddrinfo, socket};

fn host_not_found() -> io::Error {
    io::Error::new(io::ErrorKind::Other, "Host not found")
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

/// An iterator over the entries produced by a resolver.
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

fn protocol<P: IpProtocol>(ep: &IpEndpoint<P>) -> P {
    if ep.is_v4() {
        P::v4()
    } else if ep.is_v6() {
        P::v6()
    } else {
        unreachable!("");
    }
}

struct ConnectHandler<P, F>
    where P: IpProtocol,
          F: Handler<(P::Socket, IpEndpoint<P>)>,
{
    ptr: Box<(P::Socket, IpEndpoint<P>)>,
    it: ResolverIter<P>,
    handler: F,
}

impl<P, F> Handler<()> for ConnectHandler<P, F>
    where P: IpProtocol,
          F: Handler<(P::Socket, IpEndpoint<P>)>,
{
    type Output = ();

    type AsyncResult = NullAsyncResult;

    fn async_result(&self) -> Self::AsyncResult {
        NullAsyncResult
    }

    fn callback(self, io: &IoService, res: io::Result<()>) {
        let ConnectHandler { ptr, it, handler } = self;
        match res {
            Ok(_) => handler.callback(io, Ok((*ptr))),
            _ => async_connect(io, it, handler),
        }
    }
}

fn async_connect<P: IpProtocol, F: Handler<(P::Socket, IpEndpoint<P>)>>(io: &IoService, mut it: ResolverIter<P>, handler: F) {
    match it.next() {
        Some(ep) => {
            let pro = protocol(&ep);
            match socket(&pro) {
                Ok(fd) => {
                    let handler = ConnectHandler {
                        ptr: Box::new((unsafe { P::Socket::from_raw_fd(io, pro, fd) }, ep)),
                        it: it,
                        handler: handler,
                    };
                    let soc = unsafe { &*(&handler.ptr.0 as *const P::Socket) };
                    let ep = unsafe { &*(&handler.ptr.1 as *const IpEndpoint<P>) };
                    P::async_connect(&soc, &ep, handler);
                },
                Err(err) => handler.callback(io, Err(err)),
            }
        },
        None => handler.callback(io, Err(host_not_found())),
    }
}

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

    pub fn async_connect<Q: ResolverQuery<P>, F: Handler<(P::Socket, IpEndpoint<P>)>>(&self, query: Q, handler: F) {
        match self.resolve(query) {
            Ok(it) => async_connect(&self.io, it, handler),
            Err(err) => handler.callback(&self.io, Err(err)),
        }
    }

    pub fn connect<Q: ResolverQuery<P>>(&self, query: Q) -> io::Result<(P::Socket, IpEndpoint<P>)> {
        for ep in try!(self.resolve(query)) {
            let pro = protocol(&ep);
            let fd = try!(socket(&pro));
            let soc = unsafe { P::Socket::from_raw_fd(&self.io, pro, fd) };
            if let Ok(_) = P::connect(&soc, &ep) {
                return Ok((soc, ep));
            }
        }
        Err(host_not_found())
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
