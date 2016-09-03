use std::io;
use std::mem;
use {Protocol, Endpoint, RawSocket, Handler};
use backbone::{AF_UNSPEC, AF_INET, AF_INET6, SOCK_RAW, IPPROTO_ICMP, IPPROTO_ICMPV6};
use super::{IpProtocol, IpEndpoint, Resolver, ResolverIter, ResolverQuery};

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
    type Endpoint = IpEndpoint<Self>;

    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        SOCK_RAW as i32
    }

    fn protocol_type(&self) -> i32 {
        self.protocol
    }

    unsafe fn uninitialized(&self) -> Self::Endpoint {
        mem::uninitialized()
    }
}

impl IpProtocol for Icmp {
    fn is_v4(&self) -> bool {
        self == &Icmp::v4()
    }

    fn is_v6(&self) -> bool {
        self == &Icmp::v6()
    }

    fn v4() -> Icmp {
        Icmp::v4()
    }

    fn v6() -> Icmp {
        Icmp::v6()
    }

    type Socket = IcmpSocket;

    fn connect(soc: &IcmpSocket, ep: &IpEndpoint<Self>) -> io::Result<()> {
        soc.connect(ep)
    }

    fn async_connect<F: Handler<()>>(soc: &Self::Socket, ep: &IpEndpoint<Self>, handler: F) {
        soc.async_connect(ep, handler)
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

impl<'a> ResolverQuery<Icmp> for &'a str {
    fn iter(self) -> io::Result<ResolverIter<Icmp>> {
        ResolverIter::new(Icmp { family: AF_UNSPEC, protocol: 0 }, self.as_ref(), "", 0)
    }
}

/// The ICMP(v6) endpoint type.
pub type IcmpEndpoint = IpEndpoint<Icmp>;

/// The ICMP(v6) socket type.
pub type IcmpSocket = RawSocket<Icmp>;

/// The ICMP(v6) resolver type.
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
    use super::*;

    let io = IoService::new();
    let re = IcmpResolver::new(&io);
    for ep in re.resolve("127.0.0.1").unwrap() {
        assert!(ep == IcmpEndpoint::new(IpAddrV4::loopback(), 0));
    }
    for ep in re.resolve("::1").unwrap() {
        assert!(ep == IcmpEndpoint::new(IpAddrV6::loopback(), 0));
    }
    for ep in re.resolve(("localhost")).unwrap() {
        assert!(ep.addr().is_loopback());
    }
}
