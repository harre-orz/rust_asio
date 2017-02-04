use prelude::Protocol;
use ffi::{IntoI32, AF_UNSPEC, AF_INET, AF_INET6, SOCK_STREAM,
          IPPROTO_TCP, AI_PASSIVE, AI_NUMERICSERV};
use stream_socket::StreamSocket;
use socket_listener::SocketListener;
use ip::{IpProtocol, IpEndpoint, Resolver, ResolverIter, ResolverQuery, Passive};

use std::io;
use std::fmt;
use std::mem;

/// The Transmission Control Protocol.
///
/// # Examples
/// In this example, Create a TCP server socket and accept a connection by client.
///
/// ```rust,no_run
/// use asyncio::{IoContext, Protocol, Endpoint};
/// use asyncio::ip::{IpProtocol, Tcp, TcpEndpoint, TcpSocket, TcpListener};
/// use asyncio::socket_base::ReuseAddr;
///
/// let ctx = &IoContext::new().unwrap();
/// let ep = TcpEndpoint::new(Tcp::v4(), 12345);
/// let soc = TcpListener::new(ctx, ep.protocol()).unwrap();
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
/// use asyncio::{IoContext, Protocol, Endpoint};
/// use asyncio::ip::{IpProtocol, IpAddrV4, Tcp, TcpEndpoint, TcpSocket};
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// let _ = soc.connect(&TcpEndpoint::new(IpAddrV4::loopback(), 12345));
/// ```
///
/// # Examples
/// In this example, Resolve a TCP hostname and connect to TCP server.
///
/// ```rust,no_run
/// use asyncio::{IoContext, Protocol, Endpoint};
/// use asyncio::ip::{Tcp, TcpEndpoint, TcpSocket, TcpResolver};
///
/// let ctx = &IoContext::new().unwrap();
/// let re = TcpResolver::new(ctx);
/// let (soc, ep) = re.connect(("localhost", "12345")).unwrap();
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Tcp {
    family: i32,
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
        IPPROTO_TCP.i32()
    }

    unsafe fn uninitialized(&self) -> Self::Endpoint {
        mem::uninitialized()
    }
}

impl IpProtocol for Tcp {
    /// Represents a TCP for IPv4.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::Endpoint;
    /// use asyncio::ip::{IpProtocol, IpAddrV4, Tcp, TcpEndpoint};
    ///
    /// let ep = TcpEndpoint::new(IpAddrV4::any(), 0);
    /// assert_eq!(Tcp::v4(), ep.protocol());
    /// ```
    fn v4() -> Tcp {
        Tcp { family: AF_INET as i32 }
    }

    /// Represents a TCP for IPv6.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::Endpoint;
    /// use asyncio::ip::{IpProtocol, IpAddrV6, Tcp, TcpEndpoint};
    ///
    /// let ep = TcpEndpoint::new(IpAddrV6::any(), 0);
    /// assert_eq!(Tcp::v6(), ep.protocol());
    /// ```
    fn v6() -> Tcp {
        Tcp { family: AF_INET6 as i32 }
    }
}

impl fmt::Debug for Tcp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_v4() {
            write!(f, "TCPv4")
        } else if self.is_v6() {
            write!(f, "TCPv6")
        } else {
            unreachable!("Invalid address family ({}).", self.family);
        }
    }
}

impl fmt::Debug for IpEndpoint<Tcp> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Endpoint(TCP/{})", self)
    }
}

impl fmt::Debug for Resolver<Tcp, StreamSocket<Tcp>> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Resolver(TCP)")
    }
}

impl ResolverQuery<Tcp> for (Passive, u16) {
    fn iter(self) -> io::Result<ResolverIter<Tcp>> {
        let port = self.1.to_string();
        ResolverIter::new(&Tcp { family: AF_UNSPEC }, "", &port, AI_PASSIVE | AI_NUMERICSERV)
    }
}

impl<'a> ResolverQuery<Tcp> for (Passive, &'a str) {
    fn iter(self) -> io::Result<ResolverIter<Tcp>> {
        ResolverIter::new(&Tcp { family: AF_UNSPEC }, "", self.1, AI_PASSIVE)
    }
}

impl<'a, 'b> ResolverQuery<Tcp> for (&'a str, &'b str) {
    fn iter(self) -> io::Result<ResolverIter<Tcp>> {
        ResolverIter::new(&Tcp { family: AF_UNSPEC }, self.0, self.1, 0)
    }
}

/// The TCP endpoint type.
pub type TcpEndpoint = IpEndpoint<Tcp>;

/// The TCP socket type.
pub type TcpSocket = StreamSocket<Tcp>;

/// The TCP listener type.
pub type TcpListener = SocketListener<Tcp, StreamSocket<Tcp>>;

/// The TCP resolver type.
pub type TcpResolver = Resolver<Tcp, StreamSocket<Tcp>>;

#[test]
fn test_tcp() {
    assert!(Tcp::v4() == Tcp::v4());
    assert!(Tcp::v6() == Tcp::v6());
    assert!(Tcp::v4() != Tcp::v6());
}

#[test]
fn test_tcp_resolver() {
    use IoContext;
    use ip::*;

    let ctx = &IoContext::new().unwrap();
    let re = TcpResolver::new(ctx);
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
    use prelude::Endpoint;
    use core::IoContext;
    use socket_base::ReuseAddr;
    use ip::*;

    let ctx = &IoContext::new().unwrap();
    let ep = TcpEndpoint::new(IpAddrV4::any(), 12344);
    let soc = TcpSocket::new(ctx, ep.protocol()).unwrap();
    soc.set_option(ReuseAddr::new(true)).unwrap();
    soc.bind(&ep).unwrap();
    assert_eq!(soc.local_endpoint().unwrap(), ep);
}

#[test]
fn test_getsockname_v6() {
    use prelude::Endpoint;
    use core::IoContext;
    use socket_base::ReuseAddr;
    use ip::*;

    let ctx = &IoContext::new().unwrap();
    let ep = TcpEndpoint::new(IpAddrV6::any(), 12346);
    let soc = TcpSocket::new(ctx, ep.protocol()).unwrap();
    soc.set_option(ReuseAddr::new(true)).unwrap();
    soc.bind(&ep).unwrap();
    assert_eq!(soc.local_endpoint().unwrap(), ep);
}

#[test]
fn test_receive_error_when_not_connected() {
    use core::IoContext;
    use async::wrap;
    use ip::Tcp;

    use std::io;
    use std::sync::{Arc, Mutex};

    let ctx = &IoContext::new().unwrap();
    let soc = Arc::new(Mutex::new(StreamSocket::new(ctx, Tcp::v4()).unwrap()));

    let mut buf = [0; 256];
    assert!(soc.lock().unwrap().receive(&mut buf, 0).is_err());

    fn handler(_: Arc<Mutex<StreamSocket<Tcp>>>, res: io::Result<usize>) {
        assert!(res.is_err());
    }
    soc.lock().unwrap().async_receive(&mut buf, 0, wrap(handler, &soc));

    ctx.run();
}

#[test]
fn test_send_error_when_not_connected() {
    use core::IoContext;
    use async::wrap;
    use ip::Tcp;

    use std::io;
    use std::sync::{Arc, Mutex};

    let ctx = &IoContext::new().unwrap();
    let soc = Arc::new(Mutex::new(StreamSocket::new(ctx, Tcp::v4()).unwrap()));

    let mut buf = [0; 256];
    assert!(soc.lock().unwrap().send(&mut buf, 0).is_err());

    fn handler(_: Arc<Mutex<StreamSocket<Tcp>>>, res: io::Result<usize>) {
        assert!(res.is_err());
    }
    soc.lock().unwrap().async_send(&mut buf, 0, wrap(handler, &soc));

    ctx.run();
}

#[test]
fn test_format() {
    use core::IoContext;

    let ctx = &IoContext::new().unwrap();
    println!("{:?}", Tcp::v4());
    println!("{:?}", TcpEndpoint::new(Tcp::v4(), 12345));
    println!("{:?}", TcpSocket::new(ctx, Tcp::v4()).unwrap());
    println!("{:?}", TcpListener::new(ctx, Tcp::v4()).unwrap());
    println!("{:?}", TcpResolver::new(ctx));
}
