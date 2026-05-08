use crate::IoContext;
use crate::dgram_socket::{AsyncDgramSocket, DgramSocket, DgramSocketBuilder};
use crate::error::Result;
use crate::ip::resolver::Resolver;
use crate::ip::{IpEndpoint, IpProtocol};
use crate::sockaddr::AddressFamily;
use crate::socket::SocketType;
use crate::socket_base::Protocol;

/// The User Datagram Protocol.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Udp(AddressFamily);

impl Udp {
    /// Represents a UDP for IPv4.
    pub const V4: Self = Self(AddressFamily::AF_INET);

    /// Represents a UDP for IPv6.
    pub const V6: Self = Self(AddressFamily::AF_INET6);
}

impl Protocol for Udp {
    type Endpoint = IpEndpoint<Self>;
    type Type = IpProtocol;

    fn new(address_family: AddressFamily, _: Self::Type) -> Self {
        Self(address_family)
    }

    fn family_type(self) -> AddressFamily {
        self.0
    }

    fn socket_type(self) -> SocketType {
        SocketType::SOCK_DGRAM
    }

    fn protocol_type(self) -> Self::Type {
        IpProtocol::IPPROTO_UDP
    }
}

impl DgramSocket<Udp> {
    /// Creates UDP socket.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use asyncio::IoContext;
    /// use asyncio::ip::{UdpSocket, UdpEndpoint};
    /// use std::net::Ipv4Addr;
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// let ep = UdpEndpoint::v4(Ipv4Addr::LOCALHOST, 0);
    /// let soc: UdpSocket = UdpSocket::new(ctx).bind(&ep).unwrap();
    /// ```
    pub fn new(ctx: &IoContext) -> DgramSocketBuilder<Udp> {
        DgramSocketBuilder::new_impl(ctx.clone(), unsafe { IpProtocol::from_raw(0) })
    }
}

impl DgramSocketBuilder<Udp> {
    pub fn unbound(self, pro: Udp) -> Result<DgramSocket<Udp>> {
        self.unbound_impl(pro)
    }
}

impl Resolver<Udp> {
    /// The performs name resolution for UDP.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use asyncio::IoContext;
    /// use asyncio::ip::{UdpResolver, UdpEndpoint, UdpSocket};
    /// use std::net::{Ipv4Addr, Ipv6Addr};
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// let res = UdpResolver::new(ctx).resolve(("localhost", "12345")).unwrap();
    /// let soc: UdpSocket = UdpSocket::new(ctx).bind(&res).unwrap();
    /// ```
    pub fn new(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Udp(unsafe { AddressFamily::from_raw(0) }))
    }

    /// The performs name resolution for UDP with IPv4 only.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use asyncio::ip::{UdpResolver, UdpEndpoint, UdpSocket};
    /// use asyncio::IoContext;
    /// use std::net::Ipv4Addr;
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// let res = UdpResolver::v4(ctx).resolve(("localhost", "12345")).unwrap();
    /// let soc: UdpSocket = UdpSocket::new(ctx).bind(&res).unwrap();
    /// ```
    pub fn v4(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Udp::V4)
    }

    /// The performs name resolution for UDP with IPv6 only.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use asyncio::ip::{UdpResolver, UdpEndpoint, UdpSocket};
    /// use asyncio::IoContext;
    /// use std::net::Ipv6Addr;
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// let res = UdpResolver::v4(ctx).resolve(("localhost", "12345")).unwrap();
    /// let soc: UdpSocket = UdpSocket::new(ctx).bind(&res).unwrap();
    /// ```
    pub fn v6(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Udp::V6)
    }
}

/// The UDP endpoint type.
pub type UdpEndpoint = IpEndpoint<Udp>;

/// The UDP socket type.
pub type UdpSocket = DgramSocket<Udp>;

/// The UDP resolver type.
pub type UdpResolver = Resolver<Udp>;

/// The UDP async socket type.
pub type AsyncUdpSocket = AsyncDgramSocket<Udp>;

#[test]
fn test_ipv4() {
    use std::net::Ipv4Addr;

    let ep = UdpEndpoint::v4(Ipv4Addr::new(127, 0, 0, 1), 514);
    assert_eq!(ep.is_v4(), true);
    assert_eq!(ep.is_v6(), false);
    assert_eq!(ep.port(), 514);
    assert_eq!(ep.as_ip_addr(), Ipv4Addr::new(127, 0, 0, 1));
}

#[test]
fn test_ipv6() {
    use std::net::Ipv6Addr;

    let ep = UdpEndpoint::v6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 514);
    assert_eq!(ep.is_v4(), false);
    assert_eq!(ep.is_v6(), true);
    assert_eq!(ep.port(), 514);
    assert_eq!(ep.as_ip_addr(), Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1));
}

#[test]
fn test_resolver_new() {
    use crate::IoContext;
    use crate::ip::{UdpEndpoint, UdpResolver};
    use std::net::{Ipv4Addr, Ipv6Addr};

    let ctx = &IoContext::new().unwrap();
    if let Ok(res) = UdpResolver::new(ctx).resolve(("localhost", "12345")) {
        for ep in res.iter() {
            if ep.is_v4() {
                assert_eq!(ep, UdpEndpoint::v4(Ipv4Addr::LOCALHOST, 12345));
            } else if ep.is_v6() {
                assert_eq!(ep, UdpEndpoint::v6(Ipv6Addr::LOCALHOST, 12345));
            } else {
                panic!("{:?}", ep);
            }
        }
    }
}

#[test]
fn test_resolver_v4() {
    use crate::IoContext;
    use crate::ip::{UdpEndpoint, UdpResolver};
    use std::net::Ipv4Addr;

    let ctx = &IoContext::new().unwrap();
    if let Ok(res) = UdpResolver::v4(ctx).resolve(("localhost", "12345")) {
        for ep in res.iter() {
            if ep.is_v4() {
                assert_eq!(ep, UdpEndpoint::v4(Ipv4Addr::LOCALHOST, 12345));
            } else {
                panic!("{:?}", ep);
            }
        }
    }
}

#[test]
fn test_resolver_v6() {
    use crate::IoContext;
    use crate::ip::{UdpEndpoint, UdpResolver};
    use std::net::Ipv6Addr;

    let ctx = &IoContext::new().unwrap();
    if let Ok(res) = UdpResolver::v6(ctx).resolve(("localhost", "12345")) {
        for ep in res.iter() {
            if ep.is_v6() {
                let ep = ep.clone();
                assert_eq!(ep, UdpEndpoint::v6(Ipv6Addr::LOCALHOST, 12345));
            } else {
                panic!("{:?}", ep);
            }
        }
    }
}
