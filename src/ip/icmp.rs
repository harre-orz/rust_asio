use super::{IpEndpoint, IpProtocol, Resolver};
use crate::dgram::DgramSocket;
use crate::error::OsError;
use crate::executor::IoContext;
use crate::socket_base::{AddressFamily, Protocol, SocketType};

/// The Internet Control Message Protocol.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Icmp(AddressFamily, IpProtocol);

impl Icmp {
    /// Represents a ICMP.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{Icmp, IcmpEndpoint};
    /// use std::net::Ipv4Addr;
    ///
    ///
    /// let ep = IcmpEndpoint::v4(Ipv4Addr::UNSPECIFIED, 0);
    /// assert_eq!(Icmp::V4, ep.protocol());
    /// ```
    pub const V4: Self = Self(AddressFamily::INET, IpProtocol::ICMP);

    /// Represents a ICMPv6.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{Icmp, IcmpEndpoint};
    /// use std::net::Ipv6Addr;
    ///
    ///
    /// let ep = IcmpEndpoint::v6(Ipv6Addr::UNSPECIFIED, 0, 0);
    /// assert_eq!(Icmp::V6, ep.protocol());
    /// ```
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

    fn protocol_type(self) -> Self::Type {
        self.1
    }
}

/// The ICMP(v6) socket type.
pub type IcmpSocket = DgramSocket<Icmp>;

/// The ICMP(v6) endpoint type.
pub type IcmpEndpoint = IpEndpoint<Icmp>;

/// The ICMP(v6) resolver type.
pub type IcmpResolver = Resolver<Icmp>;

impl IpEndpoint<Icmp> {
    pub const fn protocol(&self) -> Icmp {
        match self.family_type() {
            AddressFamily::INET => Icmp::V4,
            AddressFamily::INET6 => Icmp::V6,
            _ => unreachable!(),
        }
    }
}

impl IcmpResolver {
    /// The performs name resolution for ICMP.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{IcmpResolver, IcmpEndpoint};
    /// use asyio::IoContext;
    /// use std::net::Ipv4Addr;
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// for ep in IcmpResolver::v4(ctx).resolve(("localhost", "")).unwrap() {
    ///     if !ep.is_v6() {
    ///         assert_eq!(ep, IcmpEndpoint::v4(Ipv4Addr::LOCALHOST, 0));
    ///     }
    ///     if !ep.is_v4() {
    ///         panic!("{:?}", ep);
    ///     }
    /// }
    /// ```
    pub fn v4(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Icmp::V4)
    }

    /// The performs name resolution for ICMPv6.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::ip::{IcmpResolver, IcmpEndpoint};
    /// use asyio::IoContext;
    /// use std::net::Ipv6Addr;
    ///
    /// let ctx = &IoContext::new().unwrap();
    /// if let Ok(it) = IcmpResolver::v6(ctx).resolve(("localhost", "")) {
    ///     for ep in it {
    ///         if !ep.is_v6() {
    ///            panic!("{:?}", ep);
    ///         }
    ///         if !ep.is_v4() {
    ///             assert_eq!(ep, IcmpEndpoint::v6(Ipv6Addr::LOCALHOST, 12345, 0));
    ///         }
    ///     }
    /// }
    pub fn v6(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Icmp::V6)
    }

    pub fn connect<I>(&self, it: I) -> Result<(IcmpSocket, I::Item), OsError>
    where
        I: Iterator<Item = IcmpEndpoint>,
    {
        let mut err = OsError::OPERATION_CANCELED;
        for ep in it {
            let soc = DgramSocket::new(self.as_ctx(), ep.protocol())?;
            match soc.connect(&ep) {
                Ok(_) => return Ok((soc, ep)),
                Err(err_) => err = err_,
            }
        }
        Err(err)
    }
}
