use super::{IpEndpoint, IpProtocol, Resolver, ResolverQuery};
use crate::dgram::{DgramSocket, DgramSocketBuilder};
use crate::error::{OsError, ResolverError};
use crate::executor::IoContext;
use crate::socket_base::{AddressFamily, Protocol, SocketType};

/// The User Datagram Protocol.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Udp(AddressFamily);

impl Udp {
    /// Represents a UDP for IPv4.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{Udp, UdpEndpoint};
    /// use std::net::Ipv4Addr;
    ///
    ///
    /// let ep = UdpEndpoint::v4(Ipv4Addr::UNSPECIFIED, 0);
    /// assert_eq!(Udp::V4, ep.protocol());
    /// ```
    pub const V4: Self = Self(AddressFamily::INET);

    /// Represents a UDP for IPv6.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{Udp, UdpEndpoint};
    /// use std::net::Ipv6Addr;
    ///
    ///
    /// let ep = UdpEndpoint::v6(Ipv6Addr::UNSPECIFIED, 0, 0);
    /// assert_eq!(Udp::V6, ep.protocol());
    /// ```
    pub const V6: Self = Self(AddressFamily::INET6);
}

impl Protocol for Udp {
    type Type = IpProtocol;
    type Endpoint = IpEndpoint<Self>;

    fn family_type(self) -> AddressFamily {
        self.0
    }

    fn socket_type(self) -> SocketType {
        SocketType::DGRAM
    }

    fn protocol_type(self) -> IpProtocol {
        IpProtocol::UDP
    }
}

/// The UDP endpoint type.
pub type UdpEndpoint = IpEndpoint<Udp>;

/// The UDP resolver type.
pub type UdpResolver = Resolver<Udp>;

/// The UDP socket type.
pub type UdpSocket<'a> = DgramSocketBuilder<'a, Udp>;

impl IpEndpoint<Udp> {
    pub const fn protocol(&self) -> Udp {
        Udp(self.family_type())
    }
}

impl UdpResolver {
    /// The performs name resolution for UDP.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{UdpResolver, UdpEndpoint};
    /// use asyio::IoContext;
    /// use std::net::{Ipv4Addr, Ipv6Addr};
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// for ep in UdpResolver::new(ctx).resolve(("localhost", "12345")).unwrap() {
    ///     if !ep.is_v6() {
    ///         assert_eq!(ep, UdpEndpoint::v4(Ipv4Addr::LOCALHOST, 12345));
    ///     }
    ///     if !ep.is_v4() {
    ///         assert_eq!(ep, UdpEndpoint::v6(Ipv6Addr::LOCALHOST, 12345, 0));
    ///     }
    /// }
    /// ```
    pub fn new(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Udp(AddressFamily::UNSPEC))
    }

    /// The performs name resolution for UDP with IPv4 only.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{UdpResolver, UdpEndpoint};
    /// use asyio::IoContext;
    /// use std::net::Ipv4Addr;
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// for ep in UdpResolver::v4(ctx).resolve(("localhost", "12345")).unwrap() {
    ///     if !ep.is_v6() {
    ///         assert_eq!(ep, UdpEndpoint::v4(Ipv4Addr::LOCALHOST, 12345));
    ///     }
    ///     if !ep.is_v4() {
    ///         panic!("{:?}", ep);
    ///     }
    /// }
    /// ```
    pub fn v4(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Udp::V4)
    }

    /// The performs name resolution for UDP with IPv6 only.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{UdpResolver, UdpEndpoint};
    /// use asyio::IoContext;
    /// use std::net::Ipv6Addr;
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// if let Ok(it) = UdpResolver::v6(ctx).resolve(("localhost", "12345")) {
    ///     for ep in it {
    ///         if !ep.is_v6() {
    ///            panic!("{:?}", ep);
    ///         }
    ///         if !ep.is_v4() {
    ///             assert_eq!(ep, UdpEndpoint::v6(Ipv6Addr::LOCALHOST, 12345, 0));
    ///         }
    ///     }
    /// }
    pub fn v6(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Udp::V6)
    }

    pub fn connect<Q>(&self, query: Q) -> Result<(DgramSocket<Udp>, UdpEndpoint), ResolverError>
    where
        Q: Into<ResolverQuery>,
    {
        let mut err = OsError::OPERATION_CANCELED;
        for ep in self.resolve(query)? {
            match DgramSocketBuilder::new(self.as_ctx(), ep.protocol()) {
                Ok(soc) => match soc.connect(ep) {
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

#[test]
fn test_ipv4() {
    use std::net::Ipv4Addr;

    let ep = UdpEndpoint::v4(Ipv4Addr::new(127, 0, 0, 1), 514);
    assert_eq!(ep.is_v4(), true);
    assert_eq!(ep.is_v6(), false);
    assert_eq!(ep.port(), 514);
    assert_eq!(ep.addr(), Ipv4Addr::new(127, 0, 0, 1));
}

#[test]
fn test_ipv6() {
    use std::net::Ipv6Addr;

    let ep = UdpEndpoint::v6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 514, 0);
    assert_eq!(ep.is_v4(), false);
    assert_eq!(ep.is_v6(), true);
    assert_eq!(ep.port(), 514);
    assert_eq!(ep.addr(), Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1));
}
