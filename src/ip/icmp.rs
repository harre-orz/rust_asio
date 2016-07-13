use std::io;
use std::fmt;
use {IoObject, Protocol, Endpoint, RawSocket};
use ip::{IpEndpoint, Resolver, ResolverQuery, ResolverIter};
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
        let soc = try!(ops::socket(&pro));
        Ok(Self::_new(io, pro, soc))
    }
}

impl fmt::Debug for RawSocket<Icmp> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "IcmpSocket")
    }
}

impl<'a, 'b> ResolverQuery<'a, Icmp> for &'b str {
    fn iter(self) -> io::Result<ResolverIter<'a, Icmp>> {
        ResolverIter::_new(Icmp { family: AF_UNSPEC, protocol: 0 }, self.as_ref(), "", 0)
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
