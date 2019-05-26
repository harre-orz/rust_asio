//

use super::{Iface, IpAddr, IpAddrV4, IpAddrV6};
use error::INVALID_ARGUMENT;
use executor::IoContext;
use socket::gethostname;
use std::fmt;
use std::io;

/// Returns a host name.
///
/// # Examples
///
/// ```
/// use asyio::IoContext;
/// use asyio::ip::host_name;
///
/// let ctx = &IoContext::new().unwrap();
/// assert_ne!(host_name(ctx).unwrap(), "")
/// ```
pub fn host_name(_: &IoContext) -> io::Result<String> {
    // IoContext を引数にとっているのは、Windowsの場合に WSAStartup() を呼び出しておく必要があるため
    Ok(gethostname()?)
}

/// Socket option determining whether outgoing multicast packets will be received on the same socket
/// if it is a member of the multicast group.
///
/// IPv4の場合は、IPPROTO_IP/IP_MULTICAST_LOOP を使い、IPv6の場合は IPPROTO_IPV6/IPV6_MULTICAST_LOOP を使う
#[derive(Clone, Debug)]
pub struct MulticastEnableLoopback(i32);

impl MulticastEnableLoopback {
    pub const fn new(on: bool) -> MulticastEnableLoopback {
        MulticastEnableLoopback(on as i32)
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }

    pub fn set(&mut self, on: bool) {
        self.0 = on as i32
    }
}

/// Socket option for time-to-live associated with outgoing multicast packets.
///
/// Implements the IPPROTO_IP/IP_MULTICAST_TTL or IPPROTO_IPV6/IPV6_MULTICAST_HOPS socket option.
///
/// # Examples
/// Setting the option:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = UdpSocket::new(ctx, Udp::v4()).unwrap();
///
/// soc.set_option(MulticastHops::new(4)).unwrap();
/// ```
///
/// Getting the option:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = UdpSocket::new(ctx, Udp::v4()).unwrap();
///
/// let opt: MulticastHops = soc.get_option().unwrap();
/// let hops: u8 = opt.get();
/// ```
#[derive(Clone, Debug)]
pub struct MulticastHops(i32);

impl MulticastHops {
    pub const fn new(ttl: u8) -> MulticastHops {
        MulticastHops(ttl as i32)
    }

    pub const fn get(&self) -> u8 {
        self.0 as u8
    }

    pub fn set(&mut self, ttl: u8) {
        self.0 = ttl as i32
    }
}

#[derive(Clone)]
pub(super) enum Mreq {
    V4(libc::ip_mreq),
    V6(libc::ipv6_mreq),
}

impl fmt::Debug for Mreq {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Mreq::V4(mreq) => write!(
                f,
                "Mreq::V4 {{ imr_multiaddr = {}, imr_interface = {} }}",
                IpAddrV4::from(mreq.imr_multiaddr),
                IpAddrV4::from(mreq.imr_interface),
            ),
            &Mreq::V6(mreq) => write!(
                f,
                "Mreq::V6 {{ ipv6mr_multiaddr = {}, ipv6mr_interface = {} }}",
                IpAddrV6::from(mreq.ipv6mr_multiaddr),
                mreq.ipv6mr_interface,
            ),
        }
    }
}

/// Socket option to join a multicast group on a specified interface.
///
/// Implements the IPPROTO_IP/IP_ADD_MEMBERSHIP or IPPROTO_IPV6/IPV6_JOIN_GROUP socket option.
///
/// # Examples
/// Setting the option:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = UdpSocket::new(ctx, Udp::v4()).unwrap();
///
/// let opt = MulticastJoinGroup::v4(IpAddrV4::new(225, 0, 0, 1), Some("lo")).unwrap();
/// soc.set_option(opt).unwrap();
/// ```
#[derive(Clone, Debug)]
pub struct MulticastJoinGroup(pub(super) Mreq);

impl MulticastJoinGroup {
    pub fn new<T>(multicast: T, if_name: Option<&str>) -> io::Result<Self>
    where
        T: Into<IpAddr>,
    {
        match multicast.into() {
            IpAddr::V4(multicast) => Self::v4(multicast, if_name),
            IpAddr::V6(multicast) => Self::v6(multicast, if_name),
        }
    }

    pub fn v4(multicast: IpAddrV4, if_name: Option<&str>) -> io::Result<Self> {
        if !multicast.is_multicast() {
            return Err(INVALID_ARGUMENT.into());
        }

        let mut interface = IpAddrV4::default();
        if let Some(if_name) = if_name {
            match Iface::new()?.get(if_name) {
                Some(iface) if !iface.ip_addr_v4.is_empty() => interface = iface.ip_addr_v4[0],
                _ => return Err(INVALID_ARGUMENT.into()),
            }
        }

        Ok(MulticastJoinGroup(Mreq::V4(libc::ip_mreq {
            imr_multiaddr: multicast.into_in_addr(),
            imr_interface: interface.into_in_addr(),
        })))
    }

    pub fn v6(multicast: IpAddrV6, if_name: Option<&str>) -> io::Result<Self> {
        if !multicast.is_multicast() {
            return Err(INVALID_ARGUMENT.into());
        }

        let mut scope_id = multicast.scope_id();
        if let Some(if_name) = if_name {
            match Iface::new()?.get(if_name) {
                Some(iface) if !iface.ip_addr_v6.is_empty() => scope_id = iface.ip_addr_v6[0].scope_id(),
                _ => return Err(INVALID_ARGUMENT.into()),
            }
        }

        Ok(MulticastJoinGroup(Mreq::V6(libc::ipv6_mreq {
            ipv6mr_multiaddr: multicast.into_in6_addr(),
            ipv6mr_interface: scope_id,
        })))
    }
}

/// Socket option to leave a multicast group on a specified interface.
///
/// Implements the IPPROTO_IP/IP_DROP_MEMBERSHIP or IPPROTO_IPV6/IPV6_LEAVE_GROUP socket option.
///
/// # Examples
/// Setting the option:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = UdpSocket::new(ctx, Udp::v4()).unwrap();
///
/// let opt = MulticastLeaveGroup::v4(IpAddrV4::new(225, 0, 0, 1), None).unwrap();
/// soc.set_option(opt).is_err();
/// ```
#[derive(Clone, Debug)]
pub struct MulticastLeaveGroup(pub(super) Mreq);

impl MulticastLeaveGroup {
    pub fn new<T>(multicast: T, if_name: Option<&str>) -> io::Result<Self>
    where
        T: Into<IpAddr>,
    {
        match multicast.into() {
            IpAddr::V4(multicast) => Self::v4(multicast, if_name),
            IpAddr::V6(multicast) => Self::v6(multicast, if_name),
        }
    }

    pub fn v4(multicast: IpAddrV4, if_name: Option<&str>) -> io::Result<Self> {
        if !multicast.is_multicast() {
            return Err(INVALID_ARGUMENT.into());
        }

        let mut interface = IpAddrV4::default();
        if let Some(if_name) = if_name {
            interface = Iface::new()?
                .get(if_name)
                .and_then(|iface| iface.ip_addr_v4.first())
                .map(|addr| addr.clone())
                .ok_or(INVALID_ARGUMENT)?;
        }

        Ok(MulticastLeaveGroup(Mreq::V4(libc::ip_mreq {
            imr_multiaddr: multicast.into_in_addr(),
            imr_interface: interface.into_in_addr(),
        })))
    }

    pub fn v6(multicast: IpAddrV6, if_name: Option<&str>) -> io::Result<Self> {
        if !multicast.is_multicast() {
            return Err(INVALID_ARGUMENT.into());
        }

        let mut scope_id = multicast.scope_id();
        if let Some(if_name) = if_name {
            scope_id = Iface::new()?
                .get(if_name)
                .and_then(|iface| iface.ip_addr_v6.first())
                .map(|addr| addr.scope_id())
                .ok_or(INVALID_ARGUMENT)?;
        }

        Ok(MulticastLeaveGroup(Mreq::V6(libc::ipv6_mreq {
            ipv6mr_multiaddr: multicast.into_in6_addr(),
            ipv6mr_interface: scope_id,
        })))
    }
}

/// Socket option for disabling the Nagle algorithm.
///
/// Implements the IPPROTO_TCP/TCP_NODELAY socket option.
///
/// # Examples
/// Setting the option:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// soc.set_option(NoDelay::new(true)).unwrap();
/// ```
///
/// Getting the option:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// let opt: NoDelay = soc.get_option().unwrap();
/// let is_set: bool = opt.get();
/// ```
#[derive(Clone, Debug)]
pub struct NoDelay(i32);

impl NoDelay {
    pub const fn new(on: bool) -> NoDelay {
        NoDelay(on as i32)
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }

    pub fn set(&mut self, on: bool) {
        self.0 = on as i32
    }
}

#[derive(Clone)]
pub(super) enum Interface {
    V4(libc::in_addr),
    V6(u32),
}

impl fmt::Debug for Interface {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Interface::V4(addr) => write!(f, "Iface::V4 {{ {:?} }}", IpAddrV4::from(addr)),
            &Interface::V6(scope_id) => write!(f, "Iface::V6 {{ {:?} }}", scope_id),
        }
    }
}

/// Socket option for local interface to use for outgoing multicast packets.
///
/// Implements the IPPROTO_IP/IP_MULTICAST_IF socket option.
///
/// # Examples
/// Setting the option:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = UdpSocket::new(ctx, Udp::v4()).unwrap();
///
/// //soc.set_option(OutboundInterface::new(IpAddr::V4(IpAddrV4::new(1,2,3,4))));
/// ```
#[derive(Clone, Debug)]
pub struct OutboundInterface(pub(super) Interface);

impl OutboundInterface {
    pub fn v4<T>(if_name: T) -> io::Result<OutboundInterface>
    where
        T: AsRef<str>,
    {
        let interface = Iface::new()?
            .get(if_name)
            .and_then(|iface| iface.ip_addr_v4.first())
            .map(|addr| addr.into_in_addr())
            .ok_or(INVALID_ARGUMENT)?;
        Ok(OutboundInterface(Interface::V4(interface)))
    }

    pub fn v6<T>(if_name: T) -> io::Result<OutboundInterface>
    where
        T: AsRef<str>,
    {
        let scope_id = Iface::new()?
            .get(if_name)
            .and_then(|iface| iface.ip_addr_v6.first())
            .map(|addr| addr.scope_id())
            .ok_or(INVALID_ARGUMENT)?;
        Ok(OutboundInterface(Interface::V6(scope_id)))
    }
}

/// Socket option for time-to-live associated with outgoing unicast packets.
///
/// Implements the IPPROTO_IP/IP_TTL or IPPROTO_IPV6/IPV6_UNICAST_HOPS socket option.
///
/// # Examples
/// Setting the option:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// soc.set_option(UnicastHops::new(4)).unwrap();
/// ```
///
/// Getting the option:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// let opt: UnicastHops = soc.get_option().unwrap();
/// assert!(opt.get() > 0)
/// ```
#[derive(Clone, Debug)]
pub struct UnicastHops(i32);

impl UnicastHops {
    pub const fn new(ttl: u8) -> UnicastHops {
        UnicastHops(ttl as i32)
    }

    pub const fn get(&self) -> u8 {
        self.0 as u8
    }

    pub fn set(&mut self, ttl: u8) {
        self.0 = ttl as i32
    }
}

/// Socket option for get/set an IPv6 socket supports IPv6 communication only.
///
/// Implements the IPPROTO_IPV6/IPV6_V6ONLY socket option.
///
/// # Examples
/// Setting the option:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpListener::new(ctx, Tcp::v6()).unwrap();
///
/// soc.set_option(V6Only::new(true)).unwrap();
/// ```
///
/// Getting the option:
///
/// ```
/// use asyio::*;
/// use asyio::ip::*;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpListener::new(ctx, Tcp::v6()).unwrap();
///
/// let opt: V6Only = soc.get_option().unwrap();
/// assert_eq!(opt.get(), false)
/// ```
#[derive(Clone, Debug)]
pub struct V6Only(i32);

impl V6Only {
    pub const fn new(on: bool) -> V6Only {
        V6Only(on as i32)
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }

    pub fn set(&mut self, on: bool) {
        self.0 = on as i32
    }
}

#[test]
fn test_host_name() {
    let ctx = &IoContext::new().unwrap();
    host_name(ctx).unwrap();
}
