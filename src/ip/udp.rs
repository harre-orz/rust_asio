use std::io;
use std::mem;
use {Protocol, DgramSocket};
use backbone::{AF_UNSPEC, AF_INET, AF_INET6, SOCK_DGRAM, AI_PASSIVE, AI_NUMERICSERV};
use super::{IpProtocol, IpEndpoint, Resolver, ResolverIter, ResolverQuery, Passive};

/// The User Datagram Protocol.
///
/// # Examples
/// In this example, Creates a UDP server socket with resolving.
///
/// ```
/// use std::io;
/// use asio::{IoService, Protocol};
/// use asio::ip::{Udp, UdpSocket, UdpEndpoint, UdpResolver, ResolverIter, Passive};
///
/// fn udp_bind(io: &IoService, it: ResolverIter<Udp>) -> io::Result<UdpSocket> {
///     for (ep, _) in it {
///         println!("{:?}", ep);
///         if let Ok(soc) = UdpSocket::new(io, ep.protocol()) {
///             if let Ok(_) = soc.bind(&ep) {
///                 return Ok(soc)
///             }
///         }
///     }
///     Err(io::Error::new(io::ErrorKind::Other, "Failed to bind"))
/// }
///
/// let io = IoService::new();
/// let re = UdpResolver::new(&io);
/// let sv = re.resolve((Passive, 12345))
///            .and_then(|it| udp_bind(&io, it))
///            .unwrap();
/// ```
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Udp {
    family: i32,
}

impl Udp {
    /// Represents a UDP for IPv4.
    pub fn v4() -> Udp {
        Udp { family: AF_INET as i32 }
    }

    /// Represents a UDP for IPv6.
    pub fn v6() -> Udp {
        Udp { family: AF_INET6 as i32 }
    }
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
        0
    }

    unsafe fn uninitialized(&self) -> Self::Endpoint {
        mem::uninitialized()
    }
}

impl IpProtocol for Udp {
    fn v4() -> Self {
        Udp::v4()
    }

    fn v6() -> Self {
        Udp::v6()
    }
}

impl IpEndpoint<Udp> {
    pub fn protocol(&self) -> Udp {
        if self.is_v4() {
            Udp::v4()
        } else if self.is_v6() {
            Udp::v6()
        } else {
            unreachable!("Invalid address family ({}).", self.ss.ss_family);
        }
    }
}

impl ResolverQuery<Udp> for (Passive, u16) {
    fn iter(self) -> io::Result<ResolverIter<Udp>> {
        let port = self.1.to_string();
        ResolverIter::new(Udp { family: AF_UNSPEC }, "", &port, AI_PASSIVE | AI_NUMERICSERV)
    }
}

impl<'a> ResolverQuery<Udp> for (Passive, &'a str) {
    fn iter(self) -> io::Result<ResolverIter<Udp>> {
        ResolverIter::new(Udp { family: AF_UNSPEC }, "", self.1, AI_PASSIVE)
    }
}

impl<'a, 'b> ResolverQuery<Udp> for (&'a str, &'b str) {
    fn iter(self) -> io::Result<ResolverIter<Udp>> {
        ResolverIter::new(Udp { family: AF_UNSPEC }, self.0, self.1, 0)
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
/// use asio::IoService;
/// use asio::ip::{Udp, UdpSocket};
///
/// let io = &IoService::new();
/// let udp4 = UdpSocket::new(io, Udp::v4()).unwrap();
/// let udp6 = UdpSocket::new(io, Udp::v6()).unwrap();
/// ```
pub type UdpSocket = DgramSocket<Udp>;

/// The UDP resolver type.
pub type UdpResolver = Resolver<Udp>;

#[test]
fn test_udp() {
    assert!(Udp::v4() == Udp::v4());
    assert!(Udp::v6() == Udp::v6());
    assert!(Udp::v4() != Udp::v6());
}

#[test]
fn test_udp_resolve() {
    use IoService;
    use super::*;

    let io = IoService::new();
    let re = UdpResolver::new(&io);
    for (ep, _) in re.resolve(("127.0.0.1", "80")).unwrap() {
        assert!(ep == UdpEndpoint::new(IpAddrV4::loopback(), 80));
    }
    for (ep, _) in re.resolve(("::1", "80")).unwrap() {
        assert!(ep == UdpEndpoint::new(IpAddrV6::loopback(), 80));
    }
    for (ep, _) in re.resolve(("localhost", "http")).unwrap() {
        assert!(ep.addr().is_loopback());
        assert!(ep.port() == 80);
    }
}
