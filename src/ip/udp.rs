use super::{IpEndpoint, IpProtocol, Resolver};
use crate::dgram::DgramSocket;
use crate::{AddressFamily, IoContext, Protocol, SocketType};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Udp(AddressFamily);

impl Udp {
    pub const V4: Self = Self(AddressFamily::INET);
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

pub type UdpEndpoint = IpEndpoint<Udp>;
pub type UdpSocket = DgramSocket<Udp>;
pub type UdpResolver = Resolver<Udp>;

impl UdpResolver {
    pub fn new(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Udp(AddressFamily::UNSPEC))
    }

    pub fn v4(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Udp::V4)
    }

    pub fn v6(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Udp::V6)
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
