use super::{IpEndpoint, IpProtocol, Resolver};
use crate::dgram::DgramSocket;
use crate::{AddressFamily, IoContext, Protocol, SocketType};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Icmp(AddressFamily, IpProtocol);

impl Icmp {
    pub const V4: Self = Self(AddressFamily::INET, IpProtocol::ICMP);
    pub const V6: Self = Self(AddressFamily::INET6, IpProtocol::ICMPV6);
}

impl Protocol for Icmp {
    type Type = IpProtocol;
    type Endpoint = IpEndpoint<Self>;

    fn family_type(self) -> AddressFamily {
        self.0
    }

    fn socket_type(self) -> SocketType {
        SocketType::RAW
    }

    fn protocol_type(self) -> IpProtocol {
        self.1
    }
}

pub type IcmpEndpoint = IpEndpoint<Icmp>;
pub type IcmpSocket = DgramSocket<Icmp>;
pub type IcmpResolver = Resolver<Icmp>;

impl IcmpResolver {
    pub fn v4(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Icmp::V4)
    }

    pub fn v6(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Icmp::V6)
    }
}
