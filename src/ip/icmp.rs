use prelude::Protocol;
use ffi::{IntoI32, AF_UNSPEC, AF_INET, AF_INET6, SOCK_RAW,
          IPPROTO_ICMP, IPPROTO_ICMPV6};
use raw_socket::RawSocket;
use ip::{IpProtocol, IpEndpoint, Resolver, ResolverIter, ResolverQuery};

use std::io;
use std::fmt;
use std::mem;

/// The Internet Control Message Protocol.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Icmp {
    family: i32,
    protocol: i32,
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
    /// Represents a ICMP.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::Endpoint;
    /// use asyncio::ip::{IpProtocol, IpAddrV4, Icmp, IcmpEndpoint};
    ///
    /// let ep = IcmpEndpoint::new(IpAddrV4::any(), 0);
    /// assert_eq!(Icmp::v4(), ep.protocol());
    /// ```
    fn v4() -> Icmp {
        Icmp { family: AF_INET as i32, protocol: IPPROTO_ICMP.i32() }
    }

    /// Represents a ICMPv6.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::Endpoint;
    /// use asyncio::ip::{IpProtocol, IpAddrV6, Icmp, IcmpEndpoint};
    ///
    /// let ep = IcmpEndpoint::new(IpAddrV6::any(), 0);
    /// assert_eq!(Icmp::v6(), ep.protocol());
    /// ```
    fn v6() -> Icmp {
        Icmp { family: AF_INET6 as i32, protocol: IPPROTO_ICMPV6.i32() }
    }
}

impl fmt::Debug for Icmp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_v4() {
            write!(f, "ICMPv4")
        } else if self.is_v6() {
            write!(f, "ICMPv6")
        } else {
            unreachable!("Invalid address family ({}).", self.family);
        }
    }
}

impl<'a> ResolverQuery<Icmp> for &'a str {
    fn iter(self) -> io::Result<ResolverIter<Icmp>> {
        ResolverIter::new(&Icmp { family: AF_UNSPEC, protocol: 0 }, self.as_ref(), "", 0)
    }
}

impl fmt::Debug for IpEndpoint<Icmp> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Endpoint(ICMP/{})", self)
    }
}

impl fmt::Debug for Resolver<Icmp, RawSocket<Icmp>> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Resolver(ICMP)")
    }
}

/// The ICMP endpoint type.
pub type IcmpEndpoint = IpEndpoint<Icmp>;

/// The ICMP socket type.
pub type IcmpSocket = RawSocket<Icmp>;

/// The ICMP resolver type.
pub type IcmpResolver = Resolver<Icmp, RawSocket<Icmp>>;

#[test]
fn test_icmp() {
    assert!(Icmp::v4() == Icmp::v4());
    assert!(Icmp::v6() == Icmp::v6());
    assert!(Icmp::v4() != Icmp::v6());
}

#[test]
fn test_icmp_resolve() {
    use core::IoContext;
    use ip::*;

    let ctx = &IoContext::new().unwrap();
    let re = IcmpResolver::new(ctx);
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

#[test]
#[ignore]
fn test_format() {
    use core::IoContext;

    let ctx = &IoContext::new().unwrap();
    println!("{:?}", Icmp::v4());
    println!("{:?}", IcmpEndpoint::new(Icmp::v4(), 12345));
    println!("{:?}", IcmpSocket::new(ctx, Icmp::v4()).unwrap());
    println!("{:?}", IcmpResolver::new(ctx));
}
