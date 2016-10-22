use std::io;
use std::mem;
use traits::{Protocol, Endpoint};
use io_service::{Handler};
use dgram_socket::{DgramSocket};
use libc::{AF_INET, AF_INET6, SOCK_DGRAM};
use super::{IpProtocol, IpEndpoint, Resolver, ResolverIter, ResolverQuery, Passive};

const AF_UNSPEC: i32 = 0;
const AI_PASSIVE: i32 = 0x0001;
//const AI_NUMERICHOST: i32 = 0x0004;
const AI_NUMERICSERV: i32 = 0x0400;

/// The User Datagram Protocol.
///
/// # Examples
/// In this example, Create a UDP client socket and send to an endpoint.
///
/// ```rust,no_run
/// use asyncio::{IoService, Protocol, Endpoint};
/// use asyncio::ip::{Udp, UdpEndpoint, UdpSocket, IpAddrV4};
///
/// let io = &IoService::new();
/// let soc = UdpSocket::new(io, Udp::v4()).unwrap();
///
/// let ep = UdpEndpoint::new(IpAddrV4::loopback(), 12345);
/// soc.send_to("hello".as_bytes(), 0, ep).unwrap();
/// ```
///
/// # Examples
/// In this example, Creates a UDP server and receive from an endpoint.
///
/// ```rust,no_run
/// use asyncio::{IoService, Protocol, Endpoint};
/// use asyncio::ip::{Udp, UdpEndpoint, UdpSocket, IpAddrV4};
/// use asyncio::socket_base::ReuseAddr;
///
/// let io = &IoService::new();
/// let ep = UdpEndpoint::new(Udp::v4(), 12345);
/// let soc = UdpSocket::new(io, ep.protocol()).unwrap();
///
/// soc.set_option(ReuseAddr::new(true)).unwrap();
/// soc.bind(&ep).unwrap();
///
/// let mut buf = [0; 256];
/// let (len, ep) = soc.receive_from(&mut buf, 0).unwrap();
/// ```
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Udp {
    family: i32,
}

impl Udp {
    /// Represents a UDP for IPv4.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::Endpoint;
    /// use asyncio::ip::{Udp, UdpEndpoint, IpAddrV4};
    ///
    /// let ep = UdpEndpoint::new(IpAddrV4::any(), 0);
    /// assert_eq!(Udp::v4(), ep.protocol());
    /// ```
    pub fn v4() -> Udp {
        Udp { family: AF_INET as i32 }
    }

    /// Represents a UDP for IPv6.
    ///
    /// Examples
    ///
    /// ```
    /// use asyncio::Endpoint;
    /// use asyncio::ip::{Udp, UdpEndpoint, IpAddrV6};
    ///
    /// let ep = UdpEndpoint::new(IpAddrV6::any(), 0);
    /// assert_eq!(Udp::v6(), ep.protocol());
    /// ```
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
    fn is_v4(&self) -> bool {
        self == &Udp::v4()
    }

    fn is_v6(&self) -> bool {
        self == &Udp::v6()
    }

    fn v4() -> Udp {
        Udp::v4()
    }

    fn v6() -> Udp {
        Udp::v6()
    }

    type Socket = UdpSocket;

    #[doc(hidden)]
    fn connect(soc: &Self::Socket, ep: &IpEndpoint<Self>) -> io::Result<()> {
        soc.connect(ep)
    }

    #[doc(hidden)]
    fn async_connect<F: Handler<(), io::Error>>(soc: &Self::Socket, ep: &IpEndpoint<Self>, handler: F) -> F::Output {
        soc.async_connect(ep, handler)
    }
}

impl Endpoint<Udp> for IpEndpoint<Udp> {
    fn protocol(&self) -> Udp {
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
/// use asyncio::IoService;
/// use asyncio::ip::{Udp, UdpSocket};
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
