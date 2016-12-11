use std::io;
use std::mem;
use traits::{Protocol, Endpoint};
use io_service::{Handler};
use stream_socket::{StreamSocket};
use socket_listener::{SocketListener};
use libc::{AF_INET, AF_INET6, SOCK_STREAM};
use super::{IpProtocol, IpEndpoint, Resolver, ResolverIter, ResolverQuery, Passive};

const AF_UNSPEC: i32 = 0;
const AI_PASSIVE: i32 = 0x0001;
//const AI_NUMERICHOST: i32 = 0x0004;
const AI_NUMERICSERV: i32 = 0x0400;

/// The Transmission Control Protocol.
///
/// # Examples
/// In this example, Create a TCP server socket and accept a connection by client.
///
/// ```rust,no_run
/// use asyncio::{IoService, Protocol, Endpoint};
/// use asyncio::ip::{Tcp, TcpEndpoint, TcpSocket, TcpListener};
/// use asyncio::socket_base::ReuseAddr;
///
/// let io = &IoService::new();
/// let ep = TcpEndpoint::new(Tcp::v4(), 12345);
/// let soc = TcpListener::new(io, ep.protocol()).unwrap();
///
/// soc.set_option(ReuseAddr::new(true)).unwrap();
/// soc.bind(&ep).unwrap();
/// soc.listen().unwrap();
///
/// let (acc, ep) = soc.accept().unwrap();
/// ```
///
/// # Examples
/// In this example, Create a TCP client socket and connect to TCP server.
///
/// ```rust,no_run
/// use asyncio::{IoService, Protocol, Endpoint};
/// use asyncio::ip::{Tcp, TcpEndpoint, TcpSocket, IpAddrV4};
///
/// let io = &IoService::new();
/// let soc = TcpSocket::new(io, Tcp::v4()).unwrap();
///
/// let _ = soc.connect(&TcpEndpoint::new(IpAddrV4::loopback(), 12345));
/// ```
///
/// # Examples
/// In this example, Resolve a TCP hostname and connect to TCP server.
///
/// ```rust,no_run
/// use asyncio::{IoService, Protocol, Endpoint};
/// use asyncio::ip::{Tcp, TcpEndpoint, TcpSocket, TcpResolver};
///
/// let io = &IoService::new();
/// let re = TcpResolver::new(io);
/// let (soc, ep) = re.connect(("localhost", "12345")).unwrap();
/// ```
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Tcp {
    family: i32,
}

impl Tcp {
    /// Represents a TCP for IPv4.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::Endpoint;
    /// use asyncio::ip::{Tcp, TcpEndpoint, IpAddrV4};
    ///
    /// let ep = TcpEndpoint::new(IpAddrV4::any(), 0);
    /// assert_eq!(Tcp::v4(), ep.protocol());
    /// ```
    pub fn v4() -> Tcp {
        Tcp { family: AF_INET as i32 }
    }

    /// Represents a TCP for IPv6.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::Endpoint;
    /// use asyncio::ip::{Tcp, TcpEndpoint, IpAddrV6};
    ///
    /// let ep = TcpEndpoint::new(IpAddrV6::any(), 0);
    /// assert_eq!(Tcp::v6(), ep.protocol());
    /// ```
    pub fn v6() -> Tcp {
        Tcp { family: AF_INET6 as i32 }
    }
}

impl Protocol for Tcp {
    type Endpoint = IpEndpoint<Self>;

    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        SOCK_STREAM as i32
    }

    fn protocol_type(&self) -> i32 {
        0
    }

    unsafe fn uninitialized(&self) -> Self::Endpoint {
        mem::uninitialized()
    }
}

impl IpProtocol for Tcp {
    fn is_v4(&self) -> bool {
        self == &Tcp::v4()
    }

    fn is_v6(&self) -> bool {
        self == &Tcp::v6()
    }

    fn v4() -> Tcp {
        Tcp::v4()
    }

    fn v6() -> Tcp {
        Tcp::v6()
    }

    type Socket = TcpSocket;

    #[doc(hidden)]
    fn connect(soc: &Self::Socket, ep: &IpEndpoint<Self>) -> io::Result<()> {
        soc.connect(ep)
    }

    #[doc(hidden)]
    fn async_connect<F: Handler<(), io::Error>>(soc: &Self::Socket, ep: &IpEndpoint<Self>, handler: F) -> F::Output {
        soc.async_connect(ep, handler)
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

impl ResolverQuery<Tcp> for (Passive, u16) {
    fn iter(self) -> io::Result<ResolverIter<Tcp>> {
        let port = self.1.to_string();
        ResolverIter::new(Tcp { family: AF_UNSPEC }, "", &port, AI_PASSIVE | AI_NUMERICSERV)
    }
}

impl<'a> ResolverQuery<Tcp> for (Passive, &'a str) {
    fn iter(self) -> io::Result<ResolverIter<Tcp>> {
        ResolverIter::new(Tcp { family: AF_UNSPEC }, "", self.1, AI_PASSIVE)
    }
}

impl<'a, 'b> ResolverQuery<Tcp> for (&'a str, &'b str) {
    fn iter(self) -> io::Result<ResolverIter<Tcp>> {
        ResolverIter::new(Tcp { family: AF_UNSPEC }, self.0, self.1, 0)
    }
}

/// The TCP endpoint type.
pub type TcpEndpoint = IpEndpoint<Tcp>;

/// The TCP socket type.
pub type TcpSocket = StreamSocket<Tcp>;

/// The TCP listener type.
pub type TcpListener = SocketListener<Tcp, StreamSocket<Tcp>>;

/// The TCP resolver type.
pub type TcpResolver = Resolver<Tcp>;

#[test]
fn test_tcp() {
    assert!(Tcp::v4() == Tcp::v4());
    assert!(Tcp::v6() == Tcp::v6());
    assert!(Tcp::v4() != Tcp::v6());
}

#[test]
fn test_tcp_resolve() {
    use IoService;
    use super::*;

    let io = IoService::new();
    let re = TcpResolver::new(&io);
    for ep in re.resolve(("127.0.0.1", "80")).unwrap() {
        assert!(ep == TcpEndpoint::new(IpAddrV4::loopback(), 80));
    }
    for ep in re.resolve(("::1", "80")).unwrap() {
        assert!(ep == TcpEndpoint::new(IpAddrV6::loopback(), 80));
    }
    for ep in re.resolve(("localhost", "http")).unwrap() {
        assert!(ep.addr().is_loopback());
        assert!(ep.port() == 80);
    }
}

#[test]
fn test_getsockname_v4() {
    use IoService;
    use socket_base::ReuseAddr;
    use super::*;

    let io = IoService::new();
    let soc = TcpSocket::new(&io, Tcp::v4()).unwrap();
    soc.set_option(ReuseAddr::new(true)).unwrap();
    let ep = TcpEndpoint::new(IpAddrV4::any(), 12345);
    soc.bind(&ep).unwrap();
    assert_eq!(soc.local_endpoint().unwrap(), ep);
}

#[test]
fn test_getsockname_v6() {
    use IoService;
    use socket_base::ReuseAddr;
    use super::*;

    let io = &IoService::new();
    let soc = TcpSocket::new(io, Tcp::v6()).unwrap();
    soc.set_option(ReuseAddr::new(true)).unwrap();
    let ep = TcpEndpoint::new(IpAddrV6::any(), 12345);
    soc.bind(&ep).unwrap();
    assert_eq!(soc.local_endpoint().unwrap(), ep);
}
