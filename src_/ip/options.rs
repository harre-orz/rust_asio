use ffi::{IPPROTO_IP, IPPROTO_IPV6, IPPROTO_TCP, IP_ADD_MEMBERSHIP, IP_DROP_MEMBERSHIP,
          IP_MULTICAST_IF, IP_TTL, IP_MULTICAST_TTL, IPV6_UNICAST_HOPS, IP_MULTICAST_LOOP,
          IPV6_JOIN_GROUP, IPV6_LEAVE_GROUP, IPV6_MULTICAST_IF, IPV6_MULTICAST_HOPS,
          IPV6_MULTICAST_LOOP, IPV6_V6ONLY, TCP_NODELAY, gethostname, in_addr, in6_addr, ip_mreq,
          ipv6_mreq};
use core::{GetSocketOption, SetSocketOption, SocketOption, IoContext};
use ip::{IpAddr, IpAddrV4, IpAddrV6, IpProtocol, Tcp};

use std::io;
use std::mem;
use libc::c_void;

fn in_addr(addr: IpAddrV4) -> in_addr {
    unsafe { mem::transmute(addr) }
}

fn in6_addr(addr: IpAddrV6) -> in6_addr {
    unsafe { mem::transmute(addr.bytes) }
}

/// Get the current host name.
///
/// # Examples
///
/// ```
/// use asyncio::IoContext;
/// use asyncio::ip::host_name;
///
/// let ctx = &IoContext::new().unwrap();
/// println!("{}", host_name(ctx).unwrap());
/// ```
pub fn host_name(_: &IoContext) -> io::Result<String> {
    Ok(gethostname()?)
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
    fn level(&self, _: &P) -> i32 {
        IPPROTO_IPV6.into()
    }

    fn name(&self, _: &P) -> i32 {
        IPV6_V6ONLY
    }
}

impl<P: IpProtocol> GetSocketOption<P> for V6Only {}

impl<P: IpProtocol> SetSocketOption<P> for V6Only {}

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
    fn level(&self, _: &Tcp) -> i32 {
        IPPROTO_TCP.into()
    }

    fn name(&self, _: &Tcp) -> i32 {
        TCP_NODELAY
    }
}

impl GetSocketOption<Tcp> for NoDelay {}

impl SetSocketOption<Tcp> for NoDelay {}

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
    fn level(&self, pro: &P) -> i32 {
        if pro == &P::v4() {
            return IPPROTO_IP.into();
        }
        if pro == &P::v6() {
            return IPPROTO_IPV6.into();
        }
        unreachable!("Invalid ip version")
    }

    fn name(&self, pro: &P) -> i32 {
        if pro == &P::v4() {
            return IP_TTL;
        }
        if pro == &P::v6() {
            return IPV6_UNICAST_HOPS;
        }
        unreachable!("Invalid ip version")
    }
}

impl<P: IpProtocol> GetSocketOption<P> for UnicastHops {}

impl<P: IpProtocol> SetSocketOption<P> for UnicastHops {}

/// Socket option determining whether outgoing multicast packets will be received on the same socket
/// if it is a member of the multicast group.
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
    fn level(&self, pro: &P) -> i32 {
        if pro == &P::v4() {
            return IPPROTO_IP.into();
        }
        if pro == &P::v6() {
            return IPPROTO_IPV6.into();
        }
        unreachable!("Invalid ip version")
    }

    fn name(&self, pro: &P) -> i32 {
        if pro == &P::v4() {
            return IP_MULTICAST_LOOP;
        }
        if pro == &P::v6() {
            return IPV6_MULTICAST_LOOP;
        }
        unreachable!("Invalid ip version")
    }
}

impl<P: IpProtocol> GetSocketOption<P> for MulticastEnableLoopback {}

impl<P: IpProtocol> SetSocketOption<P> for MulticastEnableLoopback {}

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
    fn level(&self, pro: &P) -> i32 {
        if pro == &P::v4() {
            return IPPROTO_IP.into();
        }
        if pro == &P::v6() {
            return IPPROTO_IPV6.into();
        }
        unreachable!("Invalid ip version")
    }

    fn name(&self, pro: &P) -> i32 {
        if pro == &P::v4() {
            return IP_MULTICAST_TTL;
        }
        if pro == &P::v6() {
            return IPV6_MULTICAST_HOPS;
        }
        unreachable!("Invalid ip version")
    }
}

impl<P: IpProtocol> GetSocketOption<P> for MulticastHops {}

impl<P: IpProtocol> SetSocketOption<P> for MulticastHops {}

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
/// //soc.set_option(MulticastJoinGroup::from_v4(IpAddr::V4(IpAddrV4::new(225,0,0,1)))).unwrap();
/// ```
#[derive(Clone)]
enum Mreq {
    V4(ip_mreq),
    V6(ipv6_mreq),
}

#[derive(Clone)]
pub struct MulticastJoinGroup(Mreq);

impl MulticastJoinGroup {
    pub fn new<T>(multicast: T) -> Self
    where
        T: Into<IpAddr>,
    {
        match multicast.into() {
            IpAddr::V4(multicast) => Self::v4(multicast),
            IpAddr::V6(multicast) => Self::v6(multicast),
        }
    }

    pub fn v4(multicast: IpAddrV4) -> Self {
        MulticastJoinGroup(Mreq::V4(ip_mreq {
            imr_multiaddr: in_addr(multicast),
            imr_interface: unsafe { mem::zeroed() },
        }))
    }

    pub fn v6(multicast: IpAddrV6) -> Self {
        let scope_id = multicast.scope_id();
        MulticastJoinGroup(Mreq::V6(ipv6_mreq {
            ipv6mr_multiaddr: in6_addr(multicast),
            ipv6mr_interface: scope_id,
        }))
    }
}

impl<P: IpProtocol> SocketOption<P> for MulticastJoinGroup {
    fn level(&self, pro: &P) -> i32 {
        if pro == &P::v4() {
            return IPPROTO_IP.into();
        }
        if pro == &P::v6() {
            return IPPROTO_IPV6.into();
        }
        unreachable!("Invalid ip version")
    }

    fn name(&self, pro: &P) -> i32 {
        if pro == &P::v4() {
            return IP_ADD_MEMBERSHIP;
        }
        if pro == &P::v6() {
            return IPV6_JOIN_GROUP;
        }
        unreachable!("Invalid ip version")
    }

    fn capacity(&self) -> u32 {
        mem::size_of::<ipv6_mreq>() as u32
    }
}

impl<P: IpProtocol> SetSocketOption<P> for MulticastJoinGroup {
    fn as_ptr(&self) -> *const c_void {
        match &self.0 {
            &Mreq::V4(ref mreq) => &mreq as *const _ as *const _,
            &Mreq::V6(ref mreq) => &mreq as *const _ as *const _,
        }
    }

    fn size(&self) -> u32 {
        match &self.0 {
            &Mreq::V4(ref mreq) => mem::size_of_val(mreq) as u32,
            &Mreq::V6(ref mreq) => mem::size_of_val(mreq) as u32,
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
/// //soc.set_option(MulticastLeaveGroup::new(IpAddr::V4(IpAddrV4::new(225,0,0,1))));
/// ```
#[derive(Clone)]
pub struct MulticastLeaveGroup(Mreq);

impl MulticastLeaveGroup {
    pub fn new<T>(multicast: T) -> Self
    where
        T: Into<IpAddr>,
    {
        match multicast.into() {
            IpAddr::V4(multicast) => Self::v4(multicast),
            IpAddr::V6(multicast) => Self::v6(multicast),
        }
    }

    pub fn v4(multicast: IpAddrV4) -> Self {
        MulticastLeaveGroup(Mreq::V4(ip_mreq {
            imr_multiaddr: in_addr(multicast),
            imr_interface: unsafe { mem::zeroed() },
        }))
    }

    pub fn v6(multicast: IpAddrV6) -> Self {
        let scope_id = multicast.scope_id();
        MulticastLeaveGroup(Mreq::V6(ipv6_mreq {
            ipv6mr_multiaddr: in6_addr(multicast),
            ipv6mr_interface: scope_id,
        }))
    }
}

impl<P: IpProtocol> SocketOption<P> for MulticastLeaveGroup {
    fn level(&self, pro: &P) -> i32 {
        if pro == &P::v4() {
            return IPPROTO_IP;
        }
        if pro == &P::v6() {
            return IPPROTO_IPV6;
        }
        unreachable!("Invalid ip version")
    }

    fn name(&self, pro: &P) -> i32 {
        if pro == &P::v4() {
            return IP_DROP_MEMBERSHIP;
        }
        if pro == &P::v6() {
            return IPV6_LEAVE_GROUP;
        }
        unreachable!("Invalid ip version")
    }
}

impl<P: IpProtocol> SetSocketOption<P> for MulticastLeaveGroup {
    fn as_ptr(&self) -> *const c_void {
        match &self.0 {
            &Mreq::V4(ref mreq) => mreq as *const _ as *const _,
            &Mreq::V6(ref mreq) => mreq as *const _ as *const _,
        }
    }

    fn size(&self) -> u32 {
        match &self.0 {
            &Mreq::V4(ref mreq) => mem::size_of_val(mreq) as u32,
            &Mreq::V6(ref mreq) => mem::size_of_val(mreq) as u32,
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
/// //soc.set_option(OutboundInterface::new(IpAddr::V4(IpAddrV4::new(1,2,3,4))));
/// ```
#[derive(Clone)]
pub struct OutboundInterface(Iface);

impl OutboundInterface {
    pub fn v4(interface: IpAddrV4) -> OutboundInterface {
        OutboundInterface(Iface::V4(in_addr(interface)))
    }

    pub fn v6(scope_id: u32) -> OutboundInterface {
        OutboundInterface(Iface::V6(scope_id))
    }
}

impl<P: IpProtocol> SocketOption<P> for OutboundInterface {
    fn level(&self, pro: &P) -> i32 {
        if pro == &P::v4() {
            return IPPROTO_IP;
        }
        if pro == &P::v6() {
            return IPPROTO_IPV6;
        }
        unreachable!("Invalid ip version")
    }

    fn name(&self, pro: &P) -> i32 {
        if pro == &P::v4() {
            return IP_MULTICAST_IF;
        }
        if pro == &P::v6() {
            return IPV6_MULTICAST_IF;
        }
        unreachable!("Invalid ip version")
    }

    fn capacity(&self) -> u32 {
        mem::size_of::<in_addr>() as u32
    }
}

impl<P: IpProtocol> SetSocketOption<P> for OutboundInterface {
    fn as_ptr(&self) -> *const c_void {
        match &self.0 {
            &Iface::V4(ref addr) => &addr as *const _ as *const _,
            &Iface::V6(ref scope_id) => &scope_id as *const _ as *const _,
        }
    }
}

#[test]
fn test_host_name() {
    let ctx = &IoContext::new().unwrap();
    host_name(ctx).unwrap();
}

#[test]
fn test_outbound_interface() {
    assert_eq!(mem::size_of::<u32>(), mem::size_of::<in_addr>());
}
