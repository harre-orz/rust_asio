use std::io;
use std::fmt;
use std::mem;
use std::sync::Arc;
use {IoObject, UnsafeThreadableCell, Strand, Protocol, Endpoint, RawSocket};
use ip::{IpEndpoint, Resolver, ResolverQuery, ResolverIter, UnsafeResolverIter, host_not_found};
use ops;
use ops::{AF_UNSPEC, AF_INET, AF_INET6, SOCK_RAW, IPPROTO_ICMP, IPPROTO_ICMPV6};

/// The Internet Control Message Protocol (v6).
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Icmp {
    family: i32,
    protocol: i32,
}

impl Icmp {
    /// Represents a ICMP.
    pub fn v4() -> Icmp {
        Icmp { family: AF_INET as i32, protocol: IPPROTO_ICMP }
    }

    /// Represents a ICMPv6.
    pub fn v6() -> Icmp {
        Icmp { family: AF_INET6 as i32, protocol: IPPROTO_ICMPV6 }
    }
}

impl Protocol for Icmp {
    type Endpoint = IpEndpoint<Icmp>;

    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        SOCK_RAW as i32
    }

    fn protocol_type(&self) -> i32 {
        self.protocol
    }
}

impl Endpoint<Icmp> for IpEndpoint<Icmp> {
    fn protocol(&self) -> Icmp {
        if self.is_v4() {
            Icmp::v4()
        } else if self.is_v6() {
            Icmp::v6()
        } else {
            unreachable!("Invalid address family ({}).", self.ss.ss_family);
        }
    }
}

impl RawSocket<Icmp> {
    /// Constructs a ICMP(v6) socket.
    pub fn new<T: IoObject>(io: &T, pro: Icmp) -> io::Result<RawSocket<Icmp>> {
        Ok(Self::_new(io, try!(ops::socket(pro))))
    }
}

impl fmt::Debug for RawSocket<Icmp> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "IcmpSocket")
    }
}

impl Resolver<Icmp, IcmpSocket> {
    fn async_connect_impl<F, T>(&self, mut it: UnsafeResolverIter<Icmp>, callback: F, strand: &Strand<T>)
        where F: FnOnce(Strand<T>, io::Result<(IcmpSocket, IcmpEndpoint)>) + Send + 'static,
              T: 'static {
        if let Some(e) = it.next() {
            let ep = e.endpoint();
            match IcmpSocket::new(self, ep.protocol()) {
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
        where Q: ResolverQuery<'a, Icmp>,
              F: FnOnce(Strand<T>, io::Result<(IcmpSocket, IcmpEndpoint)>) + Send + 'static,
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

    pub fn connect<'a, Q: ResolverQuery<'a, Icmp>>(&self, query: Q) -> io::Result<(IcmpSocket, IcmpEndpoint)> {
        let it = try!(query.iter());
        let mut err = host_not_found();
        for e in it {
            let ep = e.endpoint();
            let soc = try!(IcmpSocket::new(self, ep.protocol()));
            match soc.connect(&ep) {
                Ok(_) => return Ok((soc, ep)),
                Err(e) => err = e,
            }
        }
        Err(err)
    }
}

impl<'a, 'b> ResolverQuery<'a, Icmp> for &'b str {
    fn iter(self) -> io::Result<ResolverIter<'a, Icmp>> {
        ResolverIter::_new(Icmp { family: AF_UNSPEC, protocol: 0 }, self, "", 0)
    }
}

/// The ICMP(v6) endpoint type.
pub type IcmpEndpoint = IpEndpoint<Icmp>;

/// The ICMP(v6) socket type.
pub type IcmpSocket = RawSocket<Icmp>;

/// The ICMP(v6) resolver type.
pub type IcmpResolver = Resolver<Icmp, IcmpSocket>;

#[test]
fn test_icmp() {
    assert!(Icmp::v4() == Icmp::v4());
    assert!(Icmp::v6() == Icmp::v6());
    assert!(Icmp::v4() != Icmp::v6());
}

#[test]
fn test_icmp_resolve() {
    use IoService;
    use super::IpAddrV4;

    let io = IoService::new();
    let re = IcmpResolver::new(&io);
    for e in re.resolve("127.0.0.1").unwrap() {
        assert!(e.endpoint() == IcmpEndpoint::new(IpAddrV4::new(127,0,0,1), 0));
    }
}
