use super::{IpEndpoint, IpProtocol, Resolver};
use crate::listener::{ConnectedSocket, IntoConnectedSocket, SocketListener};
use crate::stream::StreamSocket;
use crate::{AddressFamily, OsError, IoContext, Protocol, SocketType, YieldContext};

/// The Transmission Control Protocol.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Tcp(AddressFamily);

impl Tcp {
    /// Represents a TCP for IPv4.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{Tcp, TcpEndpoint};
    /// use std::net::Ipv4Addr;
    ///
    ///
    /// let ep = TcpEndpoint::v4(Ipv4Addr::UNSPECIFIED, 0);
    /// assert_eq!(Tcp::V4, ep.protocol());
    /// ```
    pub const V4: Self = Self(AddressFamily::INET);

    /// Represents a TCP for IPv6.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{Tcp, TcpEndpoint};
    /// use std::net::Ipv6Addr;
    ///
    ///
    /// let ep = TcpEndpoint::v6(Ipv6Addr::UNSPECIFIED, 0, 0);
    /// assert_eq!(Tcp::V6, ep.protocol());
    /// ```
    pub const V6: Self = Self(AddressFamily::INET6);
}

impl Protocol for Tcp {
    type Type = IpProtocol;
    type Endpoint = IpEndpoint<Self>;

    fn family_type(self) -> AddressFamily {
        self.0
    }

    fn socket_type(self) -> SocketType {
        SocketType::STREAM
    }

    fn protocol_type(self) -> IpProtocol {
        IpProtocol::TCP
    }
}

/// The TCP endpoint type.
pub type TcpEndpoint = IpEndpoint<Tcp>;

/// The TCP socket type.
pub type TcpSocket = StreamSocket<Tcp>;

/// The TCP resolver type.
pub type TcpResolver = Resolver<Tcp>;

/// The TCP listener type.
pub type TcpListener = SocketListener<Tcp, TcpSocket>;

impl IpEndpoint<Tcp> {
    pub const fn protocol(&self) -> Tcp {
        Tcp(self.family_type())
    }
}

impl IntoConnectedSocket for TcpListener {
    type Socket = TcpSocket;

    fn into_connected_socket(&self, conn: ConnectedSocket) -> Self::Socket {
        TcpSocket::new_priv(conn.ctx, self.protocol(), conn.soc)
    }
}

impl TcpResolver {
    /// The performs name resolution for TCP.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{TcpResolver, TcpEndpoint};
    /// use asyio::IoContext;
    /// use std::net::{Ipv4Addr, Ipv6Addr};
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// for ep in TcpResolver::new(ctx).resolve(("localhost", "12345")).unwrap() {
    ///     if !ep.is_v6() {
    ///         assert_eq!(ep, TcpEndpoint::v4(Ipv4Addr::LOCALHOST, 12345));
    ///     }
    ///     if !ep.is_v4() {
    ///         assert_eq!(ep, TcpEndpoint::v6(Ipv6Addr::LOCALHOST, 12345, 0));
    ///     }
    /// }
    /// ```
    pub fn new(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Tcp(AddressFamily::UNSPEC))
    }

    /// The performs name resolution for TCP with IPv4 only.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{TcpResolver, TcpEndpoint};
    /// use asyio::IoContext;
    /// use std::net::Ipv4Addr;
    ///
    /// let ctx = &IoContext::new().unwrap();
    ///
    /// for ep in TcpResolver::v4(ctx).resolve(("localhost", "12345")).unwrap() {
    ///     if !ep.is_v6() {
    ///         assert_eq!(ep, TcpEndpoint::v4(Ipv4Addr::LOCALHOST, 12345));
    ///     }
    ///     if !ep.is_v4() {
    ///         panic!("{:?}", ep);
    ///     }
    /// }
    /// ```
    pub fn v4(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Tcp::V4)
    }

    /// The performs name resolution for TCP with IPv6 only.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{TcpResolver, TcpEndpoint};
    /// use asyio::IoContext;
    /// use std::net::Ipv6Addr;
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// if let Ok(it) = TcpResolver::v6(ctx).resolve(("localhost", "12345")) {
    ///     for ep in it {
    ///         if !ep.is_v6() {
    ///            panic!("{:?}", ep);
    ///         }
    ///         if !ep.is_v4() {
    ///             assert_eq!(ep, TcpEndpoint::v6(Ipv6Addr::LOCALHOST, 12345, 0));
    ///         }
    ///     }
    /// }
    /// ```
    pub fn v6(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Tcp::V6)
    }

    pub fn connect<T>(&self, it: T, yield_ctx: &mut YieldContext) -> Result<(TcpSocket, TcpEndpoint), OsError>
    where
        T: Iterator<Item = TcpEndpoint>,
    {
        let mut err = OsError::OPERATION_CANCELED;
        for ep in it {
            let soc = TcpSocket::new(self.as_ctx(), ep.protocol());
            match soc.connect(&ep, yield_ctx) {
                Ok(soc) => return Ok((soc, ep)),
                Err(err_) => err = err_,
            }
        }
        Err(err)
    }

    pub fn nb_connect<T>(&self, it: T) -> Result<(TcpSocket, TcpEndpoint), OsError>
    where
        T: Iterator<Item = TcpEndpoint>,
    {
        let mut err = OsError::OPERATION_CANCELED;
        for ep in it {
            let soc = TcpSocket::new(self.as_ctx(), ep.protocol());
            match soc.nb_connect(&ep) {
                Ok(soc) => return Ok((soc, ep)),
                Err(err_) => err = err_,
            }
        }
        Err(err)
    }
}
