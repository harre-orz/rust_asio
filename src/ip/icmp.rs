use crate::IoContext;
use crate::dgram_socket::{AsyncDgramSocket, DgramSocket, DgramSocketBuilder};
use crate::ip::resolver::Resolver;
use crate::ip::{IpEndpoint, IpProtocol};
use crate::sockaddr::AddressFamily;
use crate::socket::SocketType;
use crate::socket_base::{EndpointRef, Protocol};

/// The Internet Control Message Protocol.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Icmp(AddressFamily, i32);

impl Icmp {
    /// Represents a ICMP.
    pub const V4: Self = Self(AddressFamily::AF_INET, IpProtocol::IPPROTO_ICMP);

    /// Represents a ICMPv6.
    pub const V6: Self = Self(AddressFamily::AF_INET6, IpProtocol::IPPROTO_ICMPV6);
}

impl Protocol for Icmp {
    type Type = IpProtocol;
    type Endpoint = IpEndpoint<Self>;

    fn from_endpoint(ep: &EndpointRef<Self::Endpoint>, _: Self::Type) -> Self {
        if ep.is_v4() { Icmp::V4 } else { Icmp::V6 }
    }

    fn family_type(self) -> i32 {
        self.0.into()
    }

    fn socket_type(self) -> i32 {
        SocketType::SOCK_RAW
    }

    fn protocol_type(self) -> i32 {
        self.1
    }
}

impl DgramSocket<Icmp> {
    /// Creates ICMP socket.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use asyncio::IoContext;
    /// use asyncio::ip::{IcmpSocket, IcmpEndpoint};
    /// use std::net::Ipv4Addr;
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// let ep = IcmpEndpoint::v4(Ipv4Addr::LOCALHOST, 0);
    /// let soc: IcmpSocket = IcmpSocket::new(ctx).connect(&ep).unwrap();
    /// ```
    pub fn new(ctx: &IoContext) -> DgramSocketBuilder<Icmp> {
        DgramSocketBuilder::new_impl(ctx.clone(), IpProtocol)
    }
}
impl Resolver<Icmp> {
    /// The performs name resolution for ICMP.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use asyncio::IoContext;
    /// use asyncio::ip::{IcmpResolver, IcmpEndpoint, IcmpSocket};
    /// use std::net::Ipv4Addr;
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// let res = IcmpResolver::v4(ctx).resolve(("localhost", "")).unwrap();
    /// let soc = IcmpSocket::new(ctx).connect(&res).unwrap();
    /// ```
    pub fn v4(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Icmp::V4)
    }

    /// The performs name resolution for ICMPv6.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use asyncio::IoContext;
    /// use asyncio::ip::{IcmpResolver, IcmpEndpoint, IcmpSocket};
    /// use std::net::Ipv6Addr;
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// let res = IcmpResolver::v6(ctx).resolve(("localhost", "")).unwrap();
    /// let soc = IcmpSocket::new(ctx).connect(&res).unwrap();
    /// ```
    pub fn v6(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Icmp::V6)
    }
}

/// The ICMP(v6) endpoint type.
pub type IcmpEndpoint = IpEndpoint<Icmp>;

/// The ICMP(v6) socket type.
pub type IcmpSocket = DgramSocket<Icmp>;

/// The ICMP(v6) resolver type.
pub type IcmpResolver = Resolver<Icmp>;

/// The ICMP(v6) async socket type.
pub type AsyncIcmpSocket = AsyncDgramSocket<Icmp>;

#[test]
fn test_icmp_resolver_v4() {
    use crate::IoContext;
    use crate::ip::{IcmpEndpoint, IcmpResolver};
    use std::net::Ipv4Addr;

    let ctx = &IoContext::new().unwrap();
    if let Ok(res) = IcmpResolver::v4(ctx).resolve(("localhost", "")) {
        for ep in res.iter() {
            if ep.is_v4() {
                assert_eq!(ep, IcmpEndpoint::v4(Ipv4Addr::LOCALHOST, 0));
            } else {
                panic!("{:?}", ep);
            }
        }
    }
}

#[test]
fn test_icmp_resolver_v6() {
    use crate::IoContext;
    use crate::ip::{IcmpEndpoint, IcmpResolver};
    use std::net::Ipv6Addr;

    let ctx = &IoContext::new().unwrap();
    if let Ok(res) = IcmpResolver::v6(ctx).resolve(("localhost", "")) {
        for ep in res.iter() {
            if ep.is_v6() {
                assert_eq!(ep, IcmpEndpoint::v6(Ipv6Addr::LOCALHOST, 0));
            } else {
                panic!("{:?}", ep);
            }
        }
    }
}
