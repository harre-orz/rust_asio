use prelude::Protocol;
use ffi::{IntoI32, AF_UNSPEC, AF_INET, AF_INET6, SOCK_DGRAM,
          IPPROTO_UDP, AI_PASSIVE, AI_NUMERICSERV};
use dgram_socket::DgramSocket;
use ip::{IpProtocol, IpEndpoint, Resolver, ResolverIter, ResolverQuery, Passive};

use std::io;
use std::fmt;
use std::mem;

/// The User Datagram Protocol.
///
/// # Examples
/// In this example, Create a UDP client socket and send to an endpoint.
///
/// ```rust,no_run
/// use asyncio::{IoContext, Protocol, Endpoint};
/// use asyncio::ip::{IpProtocol, IpAddrV4, Udp, UdpEndpoint, UdpSocket};
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = UdpSocket::new(ctx, Udp::v4()).unwrap();
///
/// let ep = UdpEndpoint::new(IpAddrV4::loopback(), 12345);
/// soc.send_to("hello".as_bytes(), 0, ep).unwrap();
/// ```
///
/// # Examples
/// In this example, Creates a UDP server and receive from an endpoint.
///
/// ```rust,no_run
/// use asyncio::{IoContext, Protocol, Endpoint};
/// use asyncio::ip::{IpProtocol, IpAddrV4, Udp, UdpEndpoint, UdpSocket};
/// use asyncio::socket_base::ReuseAddr;
///
/// let ctx = &IoContext::new().unwrap();
/// let ep = UdpEndpoint::new(Udp::v4(), 12345);
/// let soc = UdpSocket::new(ctx, ep.protocol()).unwrap();
///
/// soc.set_option(ReuseAddr::new(true)).unwrap();
/// soc.bind(&ep).unwrap();
///
/// let mut buf = [0; 256];
/// let (len, ep) = soc.receive_from(&mut buf, 0).unwrap();
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Udp {
    family: i32,
}

impl Protocol for Udp {
    type Endpoint = IpEndpoint<Self>;

    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        SOCK_DGRAM as i32
    }

    fn protocol_type(&self) -> i32 {
        IPPROTO_UDP.i32()
    }

    unsafe fn uninitialized(&self) -> Self::Endpoint {
        mem::uninitialized()
    }
}

impl IpProtocol for Udp {
    /// Represents a UDP for IPv4.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::Endpoint;
    /// use asyncio::ip::{IpProtocol, IpAddrV4, Udp, UdpEndpoint};
    ///
    /// let ep = UdpEndpoint::new(IpAddrV4::any(), 0);
    /// assert_eq!(Udp::v4(), ep.protocol());
    /// ```
    fn v4() -> Udp {
        Udp { family: AF_INET as i32 }
    }

    /// Represents a UDP for IPv6.
    ///
    /// Examples
    ///
    /// ```
    /// use asyncio::Endpoint;
    /// use asyncio::ip::{IpProtocol, IpAddrV6, Udp, UdpEndpoint};
    ///
    /// let ep = UdpEndpoint::new(IpAddrV6::any(), 0);
    /// assert_eq!(Udp::v6(), ep.protocol());
    /// ```
    fn v6() -> Udp {
        Udp { family: AF_INET6 as i32 }
    }
}

impl fmt::Debug for Udp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_v4() {
            write!(f, "UDPv4")
        } else if self.is_v6() {
            write!(f, "UDPv6")
        } else {
            unreachable!("Invalid address family ({}).", self.family);
        }
    }
}

impl fmt::Debug for IpEndpoint<Udp> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Endpoint(UDP/{})", self)
    }
}

impl fmt::Debug for Resolver<Udp, DgramSocket<Udp>> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Resolver(UDP)")
    }
}

impl ResolverQuery<Udp> for (Passive, u16) {
    fn iter(self) -> io::Result<ResolverIter<Udp>> {
        let port = self.1.to_string();
        ResolverIter::new(&Udp { family: AF_UNSPEC }, "", &port, AI_PASSIVE | AI_NUMERICSERV)
    }
}

impl<'a> ResolverQuery<Udp> for (Passive, &'a str) {
    fn iter(self) -> io::Result<ResolverIter<Udp>> {
        ResolverIter::new(&Udp { family: AF_UNSPEC }, "", self.1, AI_PASSIVE)
    }
}

impl<'a, 'b> ResolverQuery<Udp> for (&'a str, &'b str) {
    fn iter(self) -> io::Result<ResolverIter<Udp>> {
        ResolverIter::new(&Udp { family: AF_UNSPEC }, self.0, self.1, 0)
    }
}

/// The UDP endpoint type.
pub type UdpEndpoint = IpEndpoint<Udp>;

/// The UDP socket type.
///
/// # Examples
/// Constructs a UDP socket.
///
/// ```
/// use asyncio::IoContext;
/// use asyncio::ip::{IpProtocol, Udp, UdpSocket};
///
/// let ctx = &IoContext::new().unwrap();
/// let udp4 = UdpSocket::new(ctx, Udp::v4()).unwrap();
/// let udp6 = UdpSocket::new(ctx, Udp::v6()).unwrap();
/// ```
pub type UdpSocket = DgramSocket<Udp>;

/// The UDP resolver type.
pub type UdpResolver = Resolver<Udp, DgramSocket<Udp>>;

#[test]
fn test_udp() {
    assert!(Udp::v4() == Udp::v4());
    assert!(Udp::v6() == Udp::v6());
    assert!(Udp::v4() != Udp::v6());
}

#[test]
fn test_udp_resolve() {
    use core::IoContext;
    use ip::*;

    let ctx = &IoContext::new().unwrap();
    let re = UdpResolver::new(ctx);
    for ep in re.resolve(("127.0.0.1", "80")).unwrap() {
        assert!(ep == UdpEndpoint::new(IpAddrV4::loopback(), 80));
    }
    for ep in re.resolve(("::1", "80")).unwrap() {
        assert!(ep == UdpEndpoint::new(IpAddrV6::loopback(), 80));
    }
    for ep in re.resolve(("localhost", "http")).unwrap() {
        assert!(ep.addr().is_loopback());
        assert!(ep.port() == 80);
    }
}


#[test]
fn test_format() {
    use core::IoContext;

    let ctx = &IoContext::new().unwrap();
    println!("{:?}", Udp::v4());
    println!("{:?}", UdpEndpoint::new(Udp::v4(), 12345));
    println!("{:?}", UdpSocket::new(ctx, Udp::v4()).unwrap());
    println!("{:?}", UdpResolver::new(ctx));
}
