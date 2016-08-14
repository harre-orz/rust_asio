use std::io;
use {Protocol, StreamSocket, SocketListener};
use backbone::{AF_UNSPEC, AF_INET, AF_INET6, SOCK_STREAM, AI_PASSIVE, AI_NUMERICSERV};
use super::{IpEndpoint, Resolver, ResolverIter, ResolverQuery, Passive};

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

impl IpEndpoint<Tcp> {
    pub fn protocol(&self) -> Tcp {
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
pub type TcpListener = SocketListener<Tcp>;

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
    for (ep, _) in re.resolve(("127.0.0.1", "80")).unwrap() {
        assert!(ep == TcpEndpoint::new(IpAddrV4::loopback(), 80));
    }
    for (ep, _) in re.resolve(("::1", "80")).unwrap() {
        assert!(ep == TcpEndpoint::new(IpAddrV6::loopback(), 80));
    }
    for (ep, _) in re.resolve(("localhost", "http")).unwrap() {
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

    let io = IoService::new();
    let soc = TcpSocket::new(&io, Tcp::v6()).unwrap();
    soc.set_option(ReuseAddr::new(true)).unwrap();
    let ep = TcpEndpoint::new(IpAddrV6::any(), 12345);
    soc.bind(&ep).unwrap();
    assert_eq!(soc.local_endpoint().unwrap(), ep);
}
