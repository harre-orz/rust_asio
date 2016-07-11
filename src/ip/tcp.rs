use std::io;
use std::fmt;
use std::mem;
use std::marker::PhantomData;
use std::sync::Arc;
use {IoObject, UnsafeThreadableCell, Strand, Protocol, Endpoint, StreamSocket, SocketListener};
use ip::{IpEndpoint, Resolver, ResolverQuery, Passive, ResolverIter, UnsafeResolverIter};
use ops;
use ops::{AF_UNSPEC, AF_INET, AF_INET6, SOCK_STREAM, AI_PASSIVE, AI_NUMERICHOST, AI_NUMERICSERV};
use ops::async::*;

fn host_not_found() -> io::Error {
    io::Error::new(io::ErrorKind::Other, "Host not found")
}

/// The Transmission Control Protocol.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Tcp {
    family: i32,
}

impl Tcp {
    /// Represents a TCP for IPv4.
    pub fn v4() -> Tcp {
        Tcp { family: AF_INET as i32 }
    }

    /// Represents a TCP for IPv6.
    pub fn v6() -> Tcp {
        Tcp { family: AF_INET6 as i32 }
    }
}

impl Protocol for Tcp {
    type Endpoint = IpEndpoint<Tcp>;

    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        SOCK_STREAM as i32
    }

    fn protocol_type(&self) -> i32 {
        0
    }
}

impl Endpoint<Tcp> for IpEndpoint<Tcp> {
    fn protocol(&self) -> Tcp {
        if self.is_v4() {
            Tcp::v4()
        } else if self.is_v6() {
            Tcp::v6()
        } else {
            unreachable!("Invalid address family ({}).", self.ss.ss_family);
        }
    }
}

impl StreamSocket<Tcp> {
    pub fn new<T: IoObject>(io: &T, pro: Tcp) -> io::Result<StreamSocket<Tcp>> {
        Ok(Self::_new(io, try!(ops::socket(pro))))
    }
}

impl fmt::Debug for StreamSocket<Tcp> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TcpSocket")
    }
}

impl SocketListener<Tcp> {
    pub fn new<T: IoObject>(io: &T, pro: Tcp) -> io::Result<SocketListener<Tcp>> {
        Ok(Self::_new(io, try!(ops::socket(pro))))
    }

    pub fn accept(&self) -> io::Result<(TcpSocket, TcpEndpoint)> {
        let (soc, ep) = try!(syncd_accept(self, unsafe { mem::uninitialized() }));
        Ok((TcpSocket::_new(self.io_service(), soc), ep))
    }

    pub fn async_accept<F, T>(&self, callback: F, strand: &Strand<T>)
        where F: FnOnce(Strand<T>, io::Result<(TcpSocket, TcpEndpoint)>) + Send + 'static,
              T: 'static {
        async_accept(self, unsafe { mem::uninitialized() }, move |obj, res| {
            match res {
                Ok((soc, ep)) => {
                    let soc = TcpSocket::_new(&obj, soc);
                    callback(obj, Ok((soc, ep)))
                }
                Err(err) => callback(obj, Err(err))
            }
        }, strand)
    }
}

impl Resolver<Tcp, TcpSocket> {
    fn async_connect_impl<F, T>(&self, mut it: UnsafeResolverIter<Tcp>, callback: F, strand: &Strand<T>)
        where F: FnOnce(Strand<T>, io::Result<(TcpSocket, TcpEndpoint)>) + Send + 'static,
              T: 'static {
        if let Some(e) = it.next() {
            let ep = e.endpoint();
            match TcpSocket::new(self, ep.protocol()) {
                Ok(soc) => {
                    let soc = Arc::new(soc);
                    mem::swap(unsafe { &mut *self.socket.get() }, &mut Some(soc.clone()));
                    let ptr = UnsafeThreadableCell::new(self as *const Self);
                    let ep_ = ep.clone();
                    soc.async_connect(&ep, move |strand, res| {
                        let re = unsafe { &**ptr };
                        let mut opt = None;
                        mem::swap(unsafe { &mut *re.socket.get() }, &mut opt);
                        let soc = Arc::try_unwrap(opt.unwrap()).unwrap();
                        match res {
                            Ok(_) =>
                                callback(strand, Ok((soc, ep_))),
                            Err(err) =>
                                if err.kind() == io::ErrorKind::Other {  // Canceled
                                    callback(strand, Err(err));
                                } else {
                                    re.async_connect_impl(it, callback, &strand);
                                }
                        }
                    }, &strand);
                },
                Err(err) => {
                    self.io_service().post_strand(move |strand| callback(strand, Err(err)), strand);
                },
            }
        } else {
            let err = host_not_found();
            self.io_service().post_strand(move |strand| callback(strand, Err(err)), strand);
        }
    }

    pub fn async_connect<'a, Q, F, T>(&self, query: Q, callback: F, strand: &Strand<T>)
        where Q: ResolverQuery<'a, Tcp>,
              F: FnOnce(Strand<T>, io::Result<(TcpSocket, TcpEndpoint)>) + Send + 'static,
              T: 'static {
        self.cancel();

        match query.iter() {
            Ok(it) => self.async_connect_impl(unsafe { it.into_inner() }, callback, strand),
            Err(err) => self.io_service().post_strand(move |strand| callback(strand, Err(err)), strand),
        }
    }

    pub fn cancel(&self) {
        let mut opt = None;
        mem::swap(unsafe { &mut *self.socket.get() }, &mut opt);
        if let Some(soc) = opt {
            soc.cancel();
        }
    }

    pub fn connect<'a, Q: ResolverQuery<'a, Tcp>>(&self, query: Q) -> io::Result<(TcpSocket, TcpEndpoint)> {
        let it = try!(query.iter());
        let mut err = host_not_found();
        for e in it {
            let ep = e.endpoint();
            let soc = try!(TcpSocket::new(self, ep.protocol()));
            match soc.connect(&ep) {
                Ok(_) => return Ok((soc, ep)),
                Err(e) => err = e,
            }
        }
        Err(err)
    }
}

impl<'a> ResolverQuery<'a, Tcp> for (Passive, u16) {
    fn iter(self) -> io::Result<ResolverIter<'a, Tcp>> {
        let port = self.1.to_string();
        ResolverIter::_new(Tcp { family: AF_UNSPEC }, "", &port, AI_PASSIVE | AI_NUMERICSERV)
    }
}

impl<'a, 'b> ResolverQuery<'a, Tcp> for (Passive, &'b str) {
    fn iter(self) -> io::Result<ResolverIter<'a, Tcp>> {
        ResolverIter::_new(Tcp { family: AF_UNSPEC }, "", self.1, AI_PASSIVE)
    }
}

impl<'a, 'b, 'c> ResolverQuery<'a, Tcp> for (&'b str, &'c str) {
    fn iter(self) -> io::Result<ResolverIter<'a, Tcp>> {
        ResolverIter::_new(Tcp { family: AF_UNSPEC }, self.0, self.1, 0)
    }
}

/// The TCP endpoint type.
pub type TcpEndpoint = IpEndpoint<Tcp>;

/// The TCP socket type.
pub type TcpSocket = StreamSocket<Tcp>;

/// The TCP listener type.
pub type TcpListener = SocketListener<Tcp>;

/// The TCP resolver type.
pub type TcpResolver = Resolver<Tcp, TcpSocket>;

#[test]
fn test_tcp() {
    assert!(Tcp::v4() == Tcp::v4());
    assert!(Tcp::v6() == Tcp::v6());
    assert!(Tcp::v4() != Tcp::v6());
}

#[test]
fn test_tcp_resolve() {
    use IoService;
    use super::IpAddrV4;

    let io = IoService::new();
    let re = TcpResolver::new(&io);
    for e in re.resolve(("127.0.0.1", "80")).unwrap() {
        assert!(e.endpoint() == TcpEndpoint::new(IpAddrV4::new(127,0,0,1), 80));
    }
}
