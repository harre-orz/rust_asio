use std::io;
use {IoObject, Strand, Protocol, Endpoint, RawSocket};
use ip::{IpEndpoint, Resolver, ResolverIter, ResolverQuery};
use ops;
use ops::{AF_UNSPEC, AF_INET, AF_INET6, SOCK_RAW, IPPROTO_ICMP, IPPROTO_ICMPV6};

/// Encapsulates the flags needed for ICMP(v6).
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Icmp {
    family: i32,
    protocol: i32,
}

impl Icmp {
    /// Makes an ICMP.
    pub fn v4() -> Icmp {
        Icmp { family: AF_INET as i32, protocol: IPPROTO_ICMP }
    }

    /// Makes an ICMPv6.
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
            unreachable!("Invalid domain ({}).", self.ss.ss_family);
        }
    }
}

impl RawSocket<Icmp> {
    pub fn new<T: IoObject>(io: &T, pro: Icmp) -> io::Result<RawSocket<Icmp>> {
        Ok(Self::_new(io, try!(ops::socket(pro))))
    }
}

impl Resolver<Icmp> {
    pub fn connect<'a, Q: ResolverQuery<'a, Icmp>>(&self, query: Q) -> io::Result<(IcmpSocket, IcmpEndpoint)> {
        let it = try!(query.iter());
        let mut err = io::Error::new(io::ErrorKind::Other, "Host not found");
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

/// The type of a ICMP(v6) endpoint.
pub type IcmpEndpoint = IpEndpoint<Icmp>;

pub type IcmpSocket = RawSocket<Icmp>;

pub type IcmpResolver = Resolver<Icmp>;

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
