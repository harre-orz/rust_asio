use crate::IoContext;
use crate::ip::resolver::Resolver;
use crate::ip::{IpEndpoint, IpProtocol};
use crate::sockaddr::AddressFamily;
use crate::socket::{Socket, SocketType};
use crate::socket_base::Protocol;
use crate::socket_listener::{
    AsyncSocketListener, ConnectedSocket, SocketListener, SocketListenerBuilder,
};
use crate::stream_socket::{AsyncStreamSocket, StreamSocket, StreamSocketBuilder};

/// The Transmission Control Protocol.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Tcp(AddressFamily);

impl Tcp {
    /// Represents a TCP for IPv4.
    pub const V4: Self = Self(AddressFamily::AF_INET);

    /// Represents a TCP for IPv6.
    pub const V6: Self = Self(AddressFamily::AF_INET6);
}

impl Protocol for Tcp {
    type Endpoint = IpEndpoint<Self>;
    type Type = IpProtocol;

    fn new(address_family: AddressFamily, _: Self::Type) -> Self {
        Self(address_family)
    }

    fn family_type(self) -> AddressFamily {
        self.0
    }

    fn socket_type(self) -> SocketType {
        SocketType::SOCK_STREAM
    }

    fn protocol_type(self) -> Self::Type {
        IpProtocol::IPPROTO_TCP
    }
}

impl ConnectedSocket<Tcp> for SocketListener<Tcp> {
    type Socket = StreamSocket<Tcp>;

    fn connected(&self, soc: Socket, pro: Tcp) -> Self::Socket {
        Self::Socket::new_impl(self.as_ctx().clone(), soc, pro)
    }
}

impl ConnectedSocket<Tcp> for AsyncSocketListener<Tcp> {
    type Socket = AsyncStreamSocket<Tcp>;

    fn connected(&self, soc: Socket, pro: Tcp) -> Self::Socket {
        StreamSocket::new_impl(self.as_ctx().clone(), soc, pro).into()
    }
}

impl StreamSocket<Tcp> {
    /// Creates TCP client socket.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::net::Ipv4Addr;
    /// use asyncio::IoContext;
    /// use asyncio::ip::{TcpEndpoint, TcpSocket};
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// let ep = TcpEndpoint::v4(Ipv4Addr::LOCALHOST, 0);
    /// let soc: TcpSocket = TcpSocket::new(ctx).connect(&ep).unwrap();
    /// ```
    pub fn new(ctx: &IoContext) -> StreamSocketBuilder<Tcp> {
        StreamSocketBuilder::new_impl(ctx.clone(), unsafe { IpProtocol::from_raw(0) })
    }
}

impl SocketListener<Tcp> {
    /// Creates TCP listener socket.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use asyncio::IoContext;
    /// use asyncio::ip::{TcpEndpoint, TcpListener};
    /// use std::net::Ipv4Addr;
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// let ep = TcpEndpoint::v4(Ipv4Addr::LOCALHOST, 0);
    /// let soc: TcpListener = TcpListener::new(ctx).listen(&ep).unwrap();
    /// ```
    pub fn new(ctx: &IoContext) -> SocketListenerBuilder<Tcp> {
        SocketListenerBuilder::new_impl(ctx.clone(), unsafe { IpProtocol::from_raw(0) })
    }
}

impl Resolver<Tcp> {
    /// The performs name resolution for TCP.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use asyncio::IoContext;
    /// use asyncio::ip::{TcpEndpoint, TcpSocket, TcpResolver};
    /// use std::net::{Ipv4Addr, Ipv6Addr};
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// let res = TcpResolver::new(ctx).resolve(("localhost", "http")).unwrap();
    /// let soc: TcpSocket = TcpSocket::new(ctx).connect(&res).unwrap();
    /// ```
    pub fn new(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Tcp(unsafe { AddressFamily::from_raw(0) }))
    }

    /// The performs name resolution for TCP with IPv4 only.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use asyncio::IoContext;
    /// use asyncio::ip::{Tcp, TcpEndpoint, TcpSocket, TcpResolver};
    /// use std::net::Ipv4Addr;
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// let res = TcpResolver::v4(ctx).resolve(("localhost", "http")).unwrap();
    /// let soc: TcpSocket = TcpSocket::new(ctx).connect(&res).unwrap();
    /// ```
    pub fn v4(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Tcp::V4)
    }

    /// The performs name resolution for TCP with IPv6 only.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use asyncio::IoContext;
    /// use asyncio::ip::{Tcp, TcpEndpoint, TcpSocket, TcpResolver};
    /// use std::net::Ipv6Addr;
    /// use asyncio::socket_base::Endpoint;
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// let res = TcpResolver::v6(ctx).resolve(("localhost", "http")).unwrap();
    /// let soc: TcpSocket = TcpSocket::new(ctx).connect(&res).unwrap();
    /// ```
    pub fn v6(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Tcp::V6)
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

/// The TCP async socket type.
pub type AsyncTcpSocket = AsyncStreamSocket<Tcp>;

/// The TCP async listener type.
pub type AsyncTcpListener = AsyncSocketListener<Tcp>;

#[test]
fn test_endpoint_v4() {
    use crate::ip::TcpEndpoint;
    use crate::socket_base::Endpoints;
    use std::net::Ipv4Addr;

    let ep = TcpEndpoint::v4(Ipv4Addr::LOCALHOST, 12345);
    assert_eq!(ep.clone().endpoints().count(), 1);
    for ep in (&ep).endpoints() {
        if ep.is_v4() {
            assert_eq!(ep, TcpEndpoint::v4(Ipv4Addr::LOCALHOST, 12345));
        } else {
            panic!("{:?}", ep);
        }
    }
    for ep in ep.endpoints() {
        if ep.is_v4() {
            assert_eq!(ep, TcpEndpoint::v4(Ipv4Addr::LOCALHOST, 12345));
        } else {
            panic!("{:?}", ep);
        }
    }
}

#[test]
fn test_endpoint_v6() {
    use crate::ip::TcpEndpoint;
    use crate::socket_base::Endpoints;
    use std::net::Ipv6Addr;

    let ep = TcpEndpoint::v6(Ipv6Addr::LOCALHOST, 12345);
    assert_eq!(ep.clone().endpoints().count(), 1);
    for ep in (&ep).endpoints() {
        if ep.is_v6() {
            assert_eq!(ep, TcpEndpoint::v6(Ipv6Addr::LOCALHOST, 12345));
        } else {
            panic!("{:?}", ep);
        }
    }
    for ep in ep.endpoints() {
        if ep.is_v6() {
            assert_eq!(ep, TcpEndpoint::v6(Ipv6Addr::LOCALHOST, 12345));
        } else {
            panic!("{:?}", ep);
        }
    }
}

#[test]
fn test_resolver_new() {
    use crate::IoContext;
    use crate::ip::{TcpEndpoint, TcpResolver};
    use std::net::{Ipv4Addr, Ipv6Addr};

    let ctx = &IoContext::new().unwrap();
    if let Ok(res) = TcpResolver::new(ctx).resolve(("localhost", "12345")) {
        for ep in res.iter() {
            if ep.is_v4() {
                assert_eq!(ep, TcpEndpoint::v4(Ipv4Addr::LOCALHOST, 12345));
            } else if ep.is_v6() {
                assert_eq!(ep, TcpEndpoint::v6(Ipv6Addr::LOCALHOST, 12345));
            } else {
                panic!("{:?}", ep);
            }
        }
        for ep in res.into_iter() {
            if ep.is_v4() {
                assert_eq!(ep, TcpEndpoint::v4(Ipv4Addr::LOCALHOST, 12345));
            } else if ep.is_v6() {
                assert_eq!(ep, TcpEndpoint::v6(Ipv6Addr::LOCALHOST, 12345));
            } else {
                panic!("{:?}", ep);
            }
        }
    }
}

#[test]
fn test_resolver_v4() {
    use crate::IoContext;
    use crate::ip::{TcpEndpoint, TcpResolver};
    use std::net::Ipv4Addr;

    let ctx = &IoContext::new().unwrap();
    if let Ok(res) = TcpResolver::v4(ctx).resolve(("localhost", "12345")) {
        for ep in res.iter() {
            if ep.is_v4() {
                assert_eq!(ep, TcpEndpoint::v4(Ipv4Addr::LOCALHOST, 12345));
            } else {
                panic!("{:?}", ep);
            }
        }
        for ep in res.into_iter() {
            if ep.is_v4() {
                assert_eq!(ep, TcpEndpoint::v4(Ipv4Addr::LOCALHOST, 12345));
            } else {
                panic!("{:?}", ep);
            }
        }
    }
}

#[test]
fn test_resolver_v6() {
    use crate::IoContext;
    use crate::ip::{TcpEndpoint, TcpResolver};
    use std::net::Ipv6Addr;

    let ctx = &IoContext::new().unwrap();
    if let Ok(res) = TcpResolver::v6(ctx).resolve(("localhost", "12345")) {
        for ep in res.iter() {
            if ep.is_v6() {
                let ep = ep.clone();
                assert_eq!(ep, TcpEndpoint::v6(Ipv6Addr::LOCALHOST, 12345));
            } else {
                panic!("{:?}", ep);
            }
        }
        for ep in res.into_iter() {
            if ep.is_v6() {
                let ep = ep.clone();
                assert_eq!(ep, TcpEndpoint::v6(Ipv6Addr::LOCALHOST, 12345));
            } else {
                panic!("{:?}", ep);
            }
        }
    }
}
