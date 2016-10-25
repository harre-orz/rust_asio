use std::io;
use std::mem;
use std::ptr;
use std::ffi::CString;
use std::marker::PhantomData;
use libc::{self, addrinfo};
use traits::{Protocol, SockAddr};
use io_service::{IoObject, IoService, FromRawFd, Handler, NoAsyncResult};
use fd_ops::socket;
use sa_ops::{SockAddrImpl};
use super::{IpProtocol, IpEndpoint};

fn host_not_found() -> io::Error {
    io::Error::new(io::ErrorKind::Other, "Host not found")
}

struct AddrInfo(*mut addrinfo);

impl AddrInfo {
    pub fn as_ptr(&self) -> *mut addrinfo {
        self.0
    }
}

impl Drop for AddrInfo {
    fn drop(&mut self) {
        unsafe { libc::freeaddrinfo(self.0) }
    }
}

fn getaddrinfo<P, N, S>(pro: P, host: N, port: S, flags: i32) -> io::Result<AddrInfo>
    where P: Protocol,
          N: Into<Vec<u8>>,
          S: Into<Vec<u8>>,
{
    let mut hints: addrinfo = unsafe { mem::zeroed() };
    hints.ai_flags = flags;
    hints.ai_family = pro.family_type();
    hints.ai_socktype = pro.socket_type();
    hints.ai_protocol = pro.protocol_type();

    let host = CString::new(host);
    let node = match &host {
        &Ok(ref node) if node.as_bytes().len() > 0
            => node.as_ptr(),
        _   => ptr::null(),
    };

    let port = CString::new(port);
    let serv = match &port {
        &Ok(ref serv) if serv.as_bytes().len() > 0
            => serv.as_ptr(),
        _   => ptr::null(),
    };

    let mut base: *mut addrinfo = ptr::null_mut();
    libc_try!(libc::getaddrinfo(node, serv, &hints, &mut base));
    Ok(AddrInfo(base))
}


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
        ResolverIter::new(self.0, self.1.as_ref(), self.2.as_ref(), 0)
    }
}

/// A query of the resolver for the passive mode.
pub struct Passive;

/// An iterator over the entries produced by a resolver.
pub struct ResolverIter<P: Protocol> {
    _base: AddrInfo,
    ai: *mut addrinfo,
    _marker: PhantomData<P>,
}

impl<P: Protocol> ResolverIter<P> {
    pub fn new(pro: P, host: &str, port: &str, flags: i32) -> io::Result<ResolverIter<P>> {
        let base = try!(getaddrinfo(pro, host, port, flags));
        let ai = base.as_ptr();
        Ok(ResolverIter {
            _base: base,
            ai: ai,
            _marker: PhantomData,
        })
    }

    pub fn next_with_flags(&mut self) -> Option<(IpEndpoint<P>, i32)> {
        while !self.ai.is_null() {
            let ai = unsafe { &mut *self.ai };
            let mut ep = IpEndpoint {
                ss: SockAddrImpl::new(0, ai.ai_addrlen as usize),
                _marker: PhantomData,
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

unsafe impl<P: Protocol> Send for ResolverIter<P> {}

fn protocol<P>(ep: &IpEndpoint<P>) -> P
    where P: IpProtocol,
{
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

    fn callback(self, io: &IoService, res: io::Result<()>) {
        let ConnectHandler { ptr, it, handler } = self;
        match res {
            Ok(_) => handler.callback(io, Ok((*ptr))),
            _ => async_connect(io, it, handler),
        }
    }

    #[doc(hidden)]
    type AsyncResult = NoAsyncResult;

    #[doc(hidden)]
    fn async_result(&self) -> Self::AsyncResult {
        NoAsyncResult
    }
}

fn async_connect<P, F>(io: &IoService, mut it: ResolverIter<P>, handler: F)
    where P: IpProtocol,
          F: Handler<(P::Socket, IpEndpoint<P>)>,
{
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
pub struct Resolver<P: Protocol> {
    io: IoService,
    _marker: PhantomData<P>,
}

impl<P: IpProtocol> Resolver<P> {
    pub fn new(io: &IoService) -> Resolver<P> {
        Resolver {
            io: io.clone(),
            _marker: PhantomData,
        }
    }

    pub fn async_connect<Q, F>(&self, query: Q, handler: F)
        where Q: ResolverQuery<P>,
              F: Handler<(P::Socket, IpEndpoint<P>)>,
    {
        match self.resolve(query) {
            Ok(it) => async_connect(&self.io, it, handler),
            Err(err) => handler.callback(&self.io, Err(err)),
        }
    }

    pub fn connect<Q>(&self, query: Q) -> io::Result<(P::Socket, IpEndpoint<P>)>
        where Q: ResolverQuery<P>,
    {
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

    pub fn resolve<Q>(&self, query: Q) -> io::Result<ResolverIter<P>>
        where Q: ResolverQuery<P>,
    {
        query.iter()
    }
}

unsafe impl<P: Protocol> IoObject for Resolver<P> {
    fn io_service(&self) -> &IoService {
        &self.io
    }
}
