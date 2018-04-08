use ffi::{AF_INET, AF_INET6, AF_UNSPEC, SOCK_STREAM, IPPROTO_TCP, AI_PASSIVE, AI_NUMERICSERV};
use core::Protocol;
use handler::Handler;
use socket_listener::SocketListener;
use stream_socket::StreamSocket;
use ip::{IpEndpoint, IpProtocol, Passive, Resolver, ResolverIter, ResolverQuery};

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
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Tcp {
    family: i32,
}

impl Protocol for Tcp {
    type Endpoint = IpEndpoint<Self>;

    type Socket = TcpSocket;

    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        SOCK_STREAM as i32
    }

    fn protocol_type(&self) -> i32 {
        IPPROTO_TCP
    }

    unsafe fn uninitialized(&self) -> Self::Endpoint {
        mem::uninitialized()
    }
}

impl IpProtocol for Tcp {
    fn async_connect<F>(soc: &Self::Socket, ep: &IpEndpoint<Self>, handler: F) -> F::Output
    where
        F: Handler<(), io::Error>,
    {
        soc.async_connect(ep, handler)
    }

    fn connect(soc: &Self::Socket, ep: &IpEndpoint<Self>) -> io::Result<()> {
        soc.connect(ep)
    }

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

impl fmt::Display for Tcp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.family_type() {
            AF_INET => write!(f, "Tcp"),
            AF_INET6 => write!(f, "Tcp6"),
            _ => unreachable!("Invalid address family ({}).", self.family),
        }
    }
}

impl ResolverQuery<Tcp> for (Passive, u16) {
    fn iter(self) -> io::Result<ResolverIter<Tcp>> {
        let port = self.1.to_string();
        ResolverIter::new(
            &Tcp { family: AF_UNSPEC },
            "",
            &port,
            AI_PASSIVE | AI_NUMERICSERV,
        )
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

/// The TCP resolver type.
pub type TcpResolver = Resolver<Tcp>;

/// The TCP listener type.
pub type TcpListener = SocketListener<Tcp>;

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
        assert_eq!(ep, TcpEndpoint::new(IpAddrV4::loopback(), 80));
    }
    for ep in re.resolve(("::1", "80")).unwrap() {
        assert_eq!(ep, TcpEndpoint::new(IpAddrV6::loopback(), 80));
    }
    for ep in re.resolve(("localhost", "http")).unwrap() {
        assert!(ep.addr().is_loopback());
        assert_eq!(ep.port(), 80);
    }
}

#[test]
fn test_getsockname_v4() {
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
    use std::sync::Arc;
    use core::IoContext;
    use handler::wrap;
    use std::io;

    let ctx = &IoContext::new().unwrap();
    let soc = Arc::new(TcpSocket::new(ctx, Tcp::v4()).unwrap());

    let mut buf = [0; 256];
    assert!(soc.receive(&mut buf, 0).is_err());

    fn handler(_: Arc<TcpSocket>, res: io::Result<usize>) {
        assert!(res.is_err());
    }
    soc.async_receive(&mut buf, 0, wrap(handler, &soc));

    ctx.run();
}

#[test]
fn test_send_error_when_not_connected() {
    use core::IoContext;
    use ip::Tcp;
    use handler::wrap;

    use std::io;
    use std::sync::Arc;

    let ctx = &IoContext::new().unwrap();
    let soc = Arc::new(StreamSocket::new(ctx, Tcp::v4()).unwrap());

    let mut buf = [0; 256];
    assert!(soc.send(&mut buf, 0).is_err());

    fn handler(_: Arc<StreamSocket<Tcp>>, res: io::Result<usize>) {
        assert!(res.is_err());
    }
    soc.async_send(&mut buf, 0, wrap(handler, &soc));

    ctx.run();
}
