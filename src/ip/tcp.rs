use super::{IpEndpoint, IpProtocol, Resolver, ResolverQuery};
use crate::error::{OsError, ResolverError};
use crate::executor::IoContext;
use crate::ffi::{ConnectedSocket, IntoSocket};
use crate::listener::{AsyncSocketListener, SocketListener, SocketListenerBuilder};
use crate::socket_base::{AddressFamily, Protocol, SocketType};
use crate::stream::{AsyncStreamSocket, StreamSocket, StreamSocketBuilder};

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

/// The TCP resolver type.
pub type TcpResolver = Resolver<Tcp>;

/// The TCP socket type.
pub type TcpSocket<'a> = StreamSocketBuilder<'a, Tcp>;

/// The TCP listener type.
pub type TcpListener<'a> = SocketListenerBuilder<'a, Tcp>;

impl IpEndpoint<Tcp> {
    pub const fn protocol(&self) -> Tcp {
        Tcp(self.family_type())
    }
}

impl IntoSocket for SocketListener<Tcp, StreamSocket<Tcp>> {
    type Socket = StreamSocket<Tcp>;

    fn into_socket(&self, soc: ConnectedSocket) -> Self::Socket {
        Self::Socket::new_priv(soc, self.protocol(), self.as_ctx())
    }
}

impl IntoSocket for AsyncSocketListener<Tcp, AsyncStreamSocket<Tcp>> {
    type Socket = AsyncStreamSocket<Tcp>;

    fn into_socket(&self, soc: ConnectedSocket) -> Self::Socket {
        let soc = self.as_ctx().async_socket(soc);
        AsyncStreamSocket::new_priv(soc, self.protocol())
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
    /// for ep in TcpResolver::new(ctx).resolve(("localhost", "http")).unwrap() {
    ///     if !ep.is_v6() {
    ///         assert_eq!(ep, TcpEndpoint::v4(Ipv4Addr::LOCALHOST, 80));
    ///     }
    ///     if !ep.is_v4() {
    ///         assert_eq!(ep, TcpEndpoint::v6(Ipv6Addr::LOCALHOST, 80, 0));
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
    /// for ep in TcpResolver::v4(ctx).resolve(("localhost", "http")).unwrap() {
    ///     if !ep.is_v6() {
    ///         assert_eq!(ep, TcpEndpoint::v4(Ipv4Addr::LOCALHOST, 80));
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
    /// if let Ok(it) = TcpResolver::v6(ctx).resolve(("localhost", "http")) {
    ///     for ep in it {
    ///         if !ep.is_v6() {
    ///            panic!("{:?}", ep);
    ///         }
    ///         if !ep.is_v4() {
    ///             assert_eq!(ep, TcpEndpoint::v6(Ipv6Addr::LOCALHOST, 80, 0));
    ///         }
    ///     }
    /// }
    /// ```
    pub fn v6(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Tcp::V6)
    }

    pub fn connect<Q>(&self, query: Q) -> Result<(StreamSocket<Tcp>, TcpEndpoint), ResolverError>
    where
        Q: Into<ResolverQuery>,
    {
        let mut err = OsError::OPERATION_CANCELED;
        for ep in self.resolve(query)? {
            match StreamSocketBuilder::new(self.as_ctx(), ep.protocol()) {
                Ok(soc) => match soc.connect(&ep) {
                    Ok(soc) => return Ok((soc, ep)),
                    Err(err_) => err = err_,
                },
                Err(err_) => {
                    err = err_;
                    break;
                }
            }
        }
        Err(ResolverError::from_os_err(err))
    }

    pub async fn async_connect<Q>(
        &self,
        query: Q,
    ) -> Result<(AsyncStreamSocket<Tcp>, TcpEndpoint), ResolverError>
    where
        Q: Into<ResolverQuery>,
    {
        let mut err = OsError::OPERATION_CANCELED;
        for ep in self.resolve(query)? {
            match StreamSocketBuilder::new(self.as_ctx(), ep.protocol()) {
                Ok(soc) => match soc.async_connect(&ep).await {
                    Ok(soc) => return Ok((soc, ep)),
                    Err(err_) => err = err_,
                },
                Err(err_) => {
                    err = err_;
                    break;
                }
            }
        }
        Err(ResolverError::from_os_err(err))
    }
}
