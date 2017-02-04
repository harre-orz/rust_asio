use prelude::{SocketOption, GetSocketOption, SetSocketOption};
use ffi::*;
use ip::{IpProtocol, IpAddrV4, IpAddrV6, IpAddr, Tcp};

use std::mem;

fn in_addr_of(addr: IpAddrV4) -> in_addr {
    unsafe { mem::transmute(addr) }
}

fn in6_addr_of(addr: IpAddrV6) -> in6_addr {
    unsafe { mem::transmute_copy(addr.as_bytes()) }
}

/// Socket option for get/set an IPv6 socket supports IPv6 communication only.
///
/// Implements the IPPROTO_IPV6/IP_V6ONLY socket option.
///
/// # Examples
/// Setting the option:
///
/// ```
/// use asyncio::*;
/// use asyncio::ip::*;
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
/// use asyncio::*;
/// use asyncio::ip::*;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpListener::new(ctx, Tcp::v6()).unwrap();
///
/// let opt: V6Only = soc.get_option().unwrap();
/// let is_set: bool = opt.get();
/// ```
#[derive(Default, Clone)]
pub struct V6Only(i32);

impl V6Only {
    pub fn new(on: bool) -> V6Only {
        V6Only(on as i32)
    }

    pub fn get(&self) -> bool {
        self.0 != 0
    }

    pub fn set(&mut self, on: bool) {
        self.0 = on as i32
    }
}

impl<P: IpProtocol> SocketOption<P> for V6Only {
    type Data = i32;

    fn level(&self, _: &P) -> i32 {
        IPPROTO_IPV6.i32()
    }

    fn name(&self, _: &P) -> i32 {
        IPV6_V6ONLY
    }

}

impl<P: IpProtocol> GetSocketOption<P> for V6Only {
    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.0
    }
}

impl<P: IpProtocol> SetSocketOption<P> for V6Only {
    fn data(&self) -> &Self::Data {
        &self.0
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
/// use asyncio::*;
/// use asyncio::ip::*;
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
/// use asyncio::*;
/// use asyncio::ip::*;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = TcpSocket::new(ctx, Tcp::v4()).unwrap();
///
/// let opt: NoDelay = soc.get_option().unwrap();
/// let is_set: bool = opt.get();
/// ```
#[derive(Default, Clone)]
pub struct NoDelay(i32);

impl NoDelay {
    pub fn new(on: bool) -> NoDelay {
        NoDelay(on as i32)
    }

    pub fn get(&self) -> bool {
        self.0 != 0
    }

    pub fn set(&mut self, on: bool) {
        self.0 = on as i32
    }
}

impl SocketOption<Tcp> for NoDelay {
    type Data = i32;

    fn level(&self, _: &Tcp) -> i32 {
        IPPROTO_TCP.i32()
    }

    fn name(&self, _: &Tcp) -> i32 {
        TCP_NODELAY
    }
}

impl GetSocketOption<Tcp> for NoDelay {
    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.0
    }
}

impl SetSocketOption<Tcp> for NoDelay {
    fn data(&self) -> &Self::Data {
        &self.0
    }
}

/// Socket option for time-to-live associated with outgoing unicast packets.
///
/// Implements the IPPROTO_IP/IP_UNICAST_TTL or IPPROTO_IPV6/IPV6_UNICAST_HOPS socket option.
///
/// # Examples
/// Setting the option:
///
/// ```
/// use asyncio::*;
/// use asyncio::ip::*;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = UdpSocket::new(ctx, Udp::v4()).unwrap();
///
/// soc.set_option(UnicastHops::new(4)).unwrap();
/// ```
///
/// Getting the option:
///
/// ```
/// use asyncio::*;
/// use asyncio::ip::*;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = UdpSocket::new(ctx, Udp::v4()).unwrap();
///
/// let opt: UnicastHops = soc.get_option().unwrap();
/// let hops: u8 = opt.get();
/// ```
#[derive(Default, Clone)]
pub struct UnicastHops(i32);

impl UnicastHops {
    pub fn new(ttl: u8) -> UnicastHops {
        UnicastHops(ttl as i32)
    }

    pub fn get(&self) -> u8 {
        self.0 as u8
    }

    pub fn set(&mut self, ttl: u8) {
        self.0 = ttl as i32
    }
}

impl<P: IpProtocol> SocketOption<P> for UnicastHops {
    type Data = i32;

    fn level(&self, pro: &P) -> i32 {
        if pro.is_v4() {
            IPPROTO_IP.i32()
        } else if pro.is_v6() {
            IPPROTO_IPV6.i32()
        } else {
            unreachable!("Invalid ip version")
        }
    }

    fn name(&self, pro: &P) -> i32 {
        if pro.is_v4() {
            IP_TTL
        } else if pro.is_v6() {
            IPV6_UNICAST_HOPS
        } else {
            unreachable!("Invalid ip version")
        }
    }
}

impl<P: IpProtocol> GetSocketOption<P> for UnicastHops {
    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.0
    }
}

impl<P: IpProtocol> SetSocketOption<P> for UnicastHops {
    fn data(&self) -> &Self::Data {
        &self.0
    }
}

/// Socket option determining whether outgoing multicast packets will be received on the same socket if it is a member of the multicast group.
///
/// Implements the IPPROTO_IP/IP_MULTICAST_LOOP or IPPROTO_IPV6/IPV6_MULTICAST_LOOP socket option.
///
/// # Examples
/// Setting the option:
///
/// ```
/// use asyncio::*;
/// use asyncio::ip::*;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = UdpSocket::new(ctx, Udp::v4()).unwrap();
///
/// soc.set_option(MulticastEnableLoopback::new(true)).unwrap();
/// ```
///
/// Getting the option:
///
/// ```
/// use asyncio::*;
/// use asyncio::ip::*;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = UdpSocket::new(ctx, Udp::v4()).unwrap();
///
/// let opt: MulticastEnableLoopback = soc.get_option().unwrap();
/// let is_set: bool = opt.get();
/// ```
#[derive(Default, Clone)]
pub struct MulticastEnableLoopback(i32);

impl MulticastEnableLoopback {
    pub fn new(on: bool) -> MulticastEnableLoopback {
        MulticastEnableLoopback(on as i32)
    }

    pub fn get(&self) -> bool {
        self.0 != 0
    }

    pub fn set(&mut self, on: bool) {
        self.0 = on as i32
    }
}

impl<P: IpProtocol> SocketOption<P> for MulticastEnableLoopback {
    type Data = i32;

    fn level(&self, pro: &P) -> i32 {
        if pro.is_v4() {
            IPPROTO_IP.i32()
        } else if pro.is_v6() {
            IPPROTO_IPV6.i32()
        } else {
            unreachable!("Invalid ip version")
        }
    }

    fn name(&self, pro: &P) -> i32 {
        if pro.is_v4() {
            IP_MULTICAST_LOOP
        } else if pro.is_v6() {
            IPV6_MULTICAST_LOOP
        } else {
            unreachable!("Invalid ip version")
        }
    }
}

impl<P: IpProtocol> GetSocketOption<P> for MulticastEnableLoopback {
    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.0
    }
}

impl<P: IpProtocol> SetSocketOption<P> for MulticastEnableLoopback {
    fn data(&self) -> &Self::Data {
        &self.0
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
/// use asyncio::*;
/// use asyncio::ip::*;
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
/// use asyncio::*;
/// use asyncio::ip::*;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = UdpSocket::new(ctx, Udp::v4()).unwrap();
///
/// let opt: MulticastHops = soc.get_option().unwrap();
/// let hops: u8 = opt.get();
/// ```
#[derive(Default, Clone)]
pub struct MulticastHops(i32);

impl MulticastHops {
    pub fn new(ttl: u8) -> MulticastHops {
        MulticastHops(ttl as i32)
    }

    pub fn get(&self) -> u8 {
        self.0 as u8
    }

    pub fn set(&mut self, ttl: u8) {
        self.0 = ttl as i32
    }
}

impl<P: IpProtocol> SocketOption<P> for MulticastHops {
    type Data = i32;

    fn level(&self, pro: &P) -> i32 {
        if pro.is_v4() {
            IPPROTO_IP.i32()
        } else if pro.is_v6() {
            IPPROTO_IPV6.i32()
        } else {
            unreachable!("Invalid ip version")
        }
    }

    fn name(&self, pro: &P) -> i32 {
        if pro.is_v4() {
            IP_MULTICAST_TTL
        } else if pro.is_v6() {
            IPV6_MULTICAST_HOPS
        } else {
            unreachable!("Invalid ip version")
        }
    }
}

impl<P: IpProtocol> GetSocketOption<P> for MulticastHops {
    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.0
    }
}

impl<P: IpProtocol> SetSocketOption<P> for MulticastHops {
    fn data(&self) -> &Self::Data {
        &self.0
    }
}

#[derive(Clone)]
enum Mreq {
    V4(ip_mreq),
    V6(ipv6_mreq),
}

/// Socket option to join a multicast group on a specified interface.
///
/// Implements the IPPROTO_IP/IP_ADD_MEMBERSHIP or IPPROTO_IPV6/IPV6_JOIN_GROUP socket option.
///
/// # Examples
/// Setting the option:
///
/// ```
/// use asyncio::*;
/// use asyncio::ip::*;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = UdpSocket::new(ctx, Udp::v4()).unwrap();
///
/// soc.set_option(MulticastJoinGroup::new(IpAddr::V4(IpAddrV4::new(225,0,0,1)))).unwrap();
/// ```
#[derive(Clone)]
pub struct MulticastJoinGroup(Mreq);

impl MulticastJoinGroup {
    pub fn new(multicast: IpAddr) -> MulticastJoinGroup {
        match multicast {
            IpAddr::V4(multicast) => Self::from_v4(multicast, IpAddrV4::any()),
            IpAddr::V6(multicast) => {
                let scope_id = multicast.get_scope_id();
                Self::from_v6(multicast, scope_id)
            }
        }
    }

    pub fn from_v4(multicast: IpAddrV4, interface: IpAddrV4) -> MulticastJoinGroup {
        MulticastJoinGroup(Mreq::V4(ip_mreq {
            imr_multiaddr: in_addr_of(multicast),
            imr_interface: in_addr_of(interface),
        }))
    }

    pub fn from_v6(multicast: IpAddrV6, scope_id: u32) -> MulticastJoinGroup {
        MulticastJoinGroup(Mreq::V6(ipv6_mreq {
            ipv6mr_multiaddr: in6_addr_of(multicast),
            ipv6mr_interface: scope_id,
        }))
    }
}

impl<P: IpProtocol> SocketOption<P> for MulticastJoinGroup {
    type Data = ();

    fn level(&self, pro: &P) -> i32 {
        if pro.is_v4() {
            IPPROTO_IP.i32()
        } else if pro.is_v6() {
            IPPROTO_IPV6.i32()
        } else {
            unreachable!("Invalid ip version")
        }
    }

    fn name(&self, pro: &P) -> i32 {
        if pro.is_v4() {
            IP_ADD_MEMBERSHIP
        } else if pro.is_v6() {
            IPV6_JOIN_GROUP
        } else {
            unreachable!("Invalid ip version")
        }
    }
}

impl<P: IpProtocol> SetSocketOption<P> for MulticastJoinGroup {
    fn data(&self) -> &Self::Data {
        match &self.0 {
            &Mreq::V4(ref mreq) => unsafe { mem::transmute(mreq) },
            &Mreq::V6(ref mreq) => unsafe { mem::transmute(mreq) },
        }
    }

    fn size(&self) -> usize {
        match &self.0 {
            &Mreq::V4(ref mreq) => mem::size_of_val(mreq),
            &Mreq::V6(ref mreq) => mem::size_of_val(mreq),
        }
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
/// use asyncio::*;
/// use asyncio::ip::*;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = UdpSocket::new(ctx, Udp::v4()).unwrap();
///
/// soc.set_option(MulticastLeaveGroup::new(IpAddr::V4(IpAddrV4::new(225,0,0,1))));
/// ```
#[derive(Clone)]
pub struct MulticastLeaveGroup(Mreq);

impl MulticastLeaveGroup {
    pub fn new(multicast: IpAddr) -> MulticastLeaveGroup {
        match multicast {
            IpAddr::V4(multicast) => Self::from_v4(multicast, IpAddrV4::any()),
            IpAddr::V6(multicast) => {
                let scope_id = multicast.get_scope_id();
                Self::from_v6(multicast, scope_id)
            }
        }
    }

    pub fn from_v4(multicast: IpAddrV4, interface: IpAddrV4) -> MulticastLeaveGroup {
        MulticastLeaveGroup(Mreq::V4(ip_mreq {
            imr_multiaddr: in_addr_of(multicast),
            imr_interface: in_addr_of(interface),
        }))
    }

    pub fn from_v6(multicast: IpAddrV6, scope_id: u32) -> MulticastLeaveGroup {
        MulticastLeaveGroup(Mreq::V6(ipv6_mreq {
            ipv6mr_multiaddr: in6_addr_of(multicast),
            ipv6mr_interface: scope_id,
        }))
    }
}

impl<P: IpProtocol> SocketOption<P> for MulticastLeaveGroup {
    type Data = ();

    fn level(&self, pro: &P) -> i32 {
        if pro == &P::v4() {
            IPPROTO_IP.i32()
        } else if pro.is_v6() {
            IPPROTO_IPV6.i32()
        } else {
            unreachable!("Invalid ip version")
        }
    }

    fn name(&self, pro: &P) -> i32 {
        if pro.is_v4() {
            IP_DROP_MEMBERSHIP
        } else if pro.is_v6() {
            IPV6_LEAVE_GROUP
        } else {
            unreachable!("Invalid ip version")
        }
    }
}

impl<P: IpProtocol> SetSocketOption<P> for MulticastLeaveGroup {
    fn data(&self) -> &Self::Data {
        match &self.0 {
            &Mreq::V4(ref mreq) => unsafe { mem::transmute(mreq) },
            &Mreq::V6(ref mreq) => unsafe { mem::transmute(mreq) },
        }
    }

    fn size(&self) -> usize {
        match &self.0 {
            &Mreq::V4(ref mreq) => mem::size_of_val(mreq),
            &Mreq::V6(ref mreq) => mem::size_of_val(mreq),
        }
    }
}

#[derive(Clone)]
enum Iface {
    V4(in_addr),
    V6(u32),
}

/// Socket option for local interface to use for outgoing multicast packets.
///
/// Implements the IPPROTO_IP/IP_MULTICAST_IF socket option.
///
/// # Examples
/// Setting the option:
///
/// ```
/// use asyncio::*;
/// use asyncio::ip::*;
///
/// let ctx = &IoContext::new().unwrap();
/// let soc = UdpSocket::new(ctx, Udp::v4()).unwrap();
///
/// soc.set_option(OutboundInterface::new(IpAddr::V4(IpAddrV4::new(1,2,3,4))));
/// ```
#[derive(Clone)]
pub struct OutboundInterface(Iface);

impl OutboundInterface {
    pub fn new(interface: IpAddr) -> OutboundInterface {
        match interface {
            IpAddr::V4(interface) => Self::from_v4(interface),
            IpAddr::V6(interface) => Self::from_v6(interface),
        }
    }

    pub fn from_v4(interface: IpAddrV4) -> OutboundInterface {
        OutboundInterface(Iface::V4(in_addr_of(interface)))
    }

    pub fn from_v6(interface: IpAddrV6) -> OutboundInterface {
        OutboundInterface(Iface::V6(interface.get_scope_id()))
    }
}

impl<P: IpProtocol> SocketOption<P> for OutboundInterface {
    type Data = u32;

    fn level(&self, pro: &P) -> i32 {
        if pro.is_v4() {
            IPPROTO_IP.i32()
        } else if pro.is_v6() {
            IPPROTO_IPV6.i32()
        } else {
            unreachable!("Invalid ip version")
        }
    }

    fn name(&self, pro: &P) -> i32 {
        if pro.is_v4() {
            IP_MULTICAST_IF
        } else if pro.is_v6() {
            IPV6_MULTICAST_IF
        } else {
            unreachable!("Invalid ip version")
        }
    }
}

impl<P: IpProtocol> SetSocketOption<P> for OutboundInterface {
    fn data(&self) -> &Self::Data {
        match &self.0 {
            &Iface::V4(ref addr) => unsafe { mem::transmute(addr) },
            &Iface::V6(ref scope_id) => &scope_id,
        }
    }
}

#[test]
fn test_outbound_interface() {
    assert_eq!(mem::size_of::<u32>(), mem::size_of::<in_addr>());
}
