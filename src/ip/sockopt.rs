use crate::ip::endpoint::Ip;
use crate::ip::{IpProtocol, Tcp};
use crate::socket_base::{GetSockOpt, Protocol, SetSockOpt, SockOpt};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::num::NonZeroU8;
use std::{ptr, slice};
use crate::iface::Iface;

pub struct NoDelay(libc::c_int);

impl NoDelay {
    pub const ON: Self = Self(1);
    pub const OFF: Self = Self(0);

    pub const fn new(on: bool) -> Self {
        Self(on as i32)
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }
}

impl SockOpt<Tcp> for NoDelay {
    fn key(_: Tcp) -> (libc::c_int, libc::c_int) {
        (libc::IPPROTO_TCP, libc::TCP_NODELAY)
    }
}

impl SetSockOpt<Tcp> for NoDelay {}

impl GetSockOpt<Tcp> for NoDelay {}

pub struct V6Only(libc::c_int);

impl V6Only {
    pub const ON: Self = Self(1);
    pub const OFF: Self = Self(0);

    pub const fn new(on: bool) -> Self {
        Self(on as i32)
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }
}

impl<P> SockOpt<P> for V6Only
where
    P: Protocol<Type = IpProtocol>,
{
    fn key(_: P) -> (libc::c_int, libc::c_int) {
        (libc::IPPROTO_IPV6, libc::IPV6_V6ONLY)
    }
}

pub struct UcastHops(libc::c_int);

impl UcastHops {
    pub const fn new(hops: NonZeroU8) -> Self {
        Self(hops.get() as libc::c_int)
    }

    pub const fn get(&self) -> u8 {
        self.0 as u8
    }
}

impl<P> SockOpt<P> for UcastHops
where
    P: Protocol<Type = IpProtocol>,
{
    fn key(pro: P) -> (libc::c_int, libc::c_int) {
        match P::Type::version(pro) {
            Ip::V4 => (libc::IPPROTO_IP, libc::IP_TTL),
            Ip::V6 => (libc::IPPROTO_IPV6, libc::IPV6_UNICAST_HOPS),
        }
    }
}

pub struct McastLoop(libc::c_int);

impl McastLoop {
    pub const ON: Self = Self::new(true);
    pub const OFF: Self = Self::new(false);

    pub const fn new(on: bool) -> Self {
        Self(on as libc::c_int)
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }
}

impl<P> SockOpt<P> for McastLoop
where
    P: Protocol<Type = IpProtocol>,
{
    fn key(pro: P) -> (libc::c_int, libc::c_int) {
        match P::Type::version(pro) {
            Ip::V4 => (libc::IPPROTO_IP, libc::IP_MULTICAST_LOOP),
            Ip::V6 => (libc::IPPROTO_IPV6, libc::IPV6_MULTICAST_LOOP),
        }
    }
}

pub struct McastHops(libc::c_int);

impl McastHops {
    pub const fn new(hops: NonZeroU8) -> Self {
        Self(hops.get() as libc::c_int)
    }

    pub const fn get(&self) -> u8 {
        self.0 as u8
    }
}

impl<P> SockOpt<P> for McastHops
where
    P: Protocol<Type = IpProtocol>,
{
    fn key(pro: P) -> (libc::c_int, libc::c_int) {
        match P::Type::version(pro) {
            Ip::V4 => (libc::IPPROTO_IP, libc::IP_MULTICAST_TTL),
            Ip::V6 => (libc::IPPROTO_IPV6, libc::IPV6_MULTICAST_HOPS),
        }
    }
}

enum McastReq {
    #[cfg(target_os = "linux")]
    V4(libc::ip_mreqn),
    #[cfg(not(target_os = "linux"))]
    V4(libc::ip_mreq),
    V6(libc::ipv6_mreq),
}

pub struct McastJoinGroup(McastReq);

impl McastJoinGroup {
    pub const fn new(mcast_addr: &IpAddr) -> Self
    {
        match mcast_addr {
            IpAddr::V4(mcast_addr) => Self::v4(mcast_addr),
            IpAddr::V6(mcast_addr) => Self::v6(mcast_addr),
        }
    }

    pub const fn v4(mcast_addr: &Ipv4Addr) -> Self
    {
        Self(McastReq::V4(libc::ip_mreqn {
            imr_multiaddr: libc::in_addr { s_addr: mcast_addr.to_bits() },
            imr_address: libc::in_addr { s_addr: 0 },
            imr_ifindex: 0,
        }))
    }

    pub const fn v6(mcast_addr: &Ipv6Addr) -> Self
    {
        Self(McastReq::V6(libc::ipv6_mreq {
            ipv6mr_multiaddr: libc::in6_addr { s6_addr: mcast_addr.octets() },
            ipv6mr_interface: 0,
        }))
    }
}

impl<P> SockOpt<P> for McastJoinGroup
where
    P: Protocol<Type = IpProtocol>,
{
    fn key(pro: P) -> (libc::c_int, libc::c_int) {
        match P::Type::version(pro) {
            Ip::V4 => (libc::IPPROTO_IP, libc::IP_ADD_MEMBERSHIP),
            Ip::V6 => (libc::IPPROTO_IPV6, libc::IPV6_ADD_MEMBERSHIP),
        }
    }
}

impl<P> SetSockOpt<P> for McastJoinGroup
where
    P: Protocol<Type = IpProtocol>,
{
    fn data(&self, pro: P) -> (libc::c_int, libc::c_int, &[u8]) {
        match (P::Type::version(pro), &self.0) {
            (Ip::V4, McastReq::V4(imr)) => {
                let bytes = unsafe {
                    slice::from_raw_parts(ptr::from_ref(imr).cast(), size_of_val(imr))
                };
                (libc::IPPROTO_IP, libc::IP_ADD_MEMBERSHIP, bytes)
            },
            (Ip::V6, McastReq::V6(ipv6mr)) => {
                let bytes = unsafe {
                    slice::from_raw_parts(ptr::from_ref(ipv6mr).cast(), size_of_val(ipv6mr))
                };
                (libc::IPPROTO_IPV6, libc::IPV6_ADD_MEMBERSHIP, bytes)
            },
            _ => (0, 0, &[]),
        }
    }
}

pub struct McastLeaveGroup(McastReq);

impl McastLeaveGroup {
    pub const fn new(mcast_addr: &IpAddr) -> Self
    {
        match mcast_addr {
            IpAddr::V4(mcast_addr) => Self::v4(mcast_addr),
            IpAddr::V6(mcast_addr) => Self::v6(mcast_addr),
        }
    }

    pub const fn v4(mcast_addr: &Ipv4Addr) -> Self
    {
        Self(McastReq::V4(libc::ip_mreqn {
            imr_multiaddr: libc::in_addr { s_addr: mcast_addr.to_bits() },
            imr_address: libc::in_addr { s_addr: 0 },
            imr_ifindex: 0,
        }))
    }

    pub const fn v6(mcast_addr: &Ipv6Addr) -> Self
    {
        Self(McastReq::V6(libc::ipv6_mreq {
            ipv6mr_multiaddr: libc::in6_addr { s6_addr: mcast_addr.octets() },
            ipv6mr_interface: 0,
        }))
    }
}

impl<P> SockOpt<P> for McastLeaveGroup
where
    P: Protocol<Type = IpProtocol>,
{
    fn key(pro: P) -> (libc::c_int, libc::c_int) {
        match P::Type::version(pro) {
            Ip::V4 => (libc::IPPROTO_IP, libc::IP_DROP_MEMBERSHIP),
            Ip::V6 => (libc::IPPROTO_IPV6, libc::IPV6_DROP_MEMBERSHIP),
        }
    }
}

impl<P> SetSockOpt<P> for McastLeaveGroup
where
    P: Protocol<Type = IpProtocol>,
{
    fn data(&self, pro: P) -> (libc::c_int, libc::c_int, &[u8]) {
        match (P::Type::version(pro), &self.0) {
            (Ip::V4, McastReq::V4(imr)) => {
                let bytes = unsafe {
                    slice::from_raw_parts(ptr::from_ref(imr).cast(), size_of_val(imr))
                };
                (libc::IPPROTO_IP, libc::IP_DROP_MEMBERSHIP, bytes)
            },
            (Ip::V6, McastReq::V6(ipv6mr)) => {
                let bytes = unsafe {
                    slice::from_raw_parts(ptr::from_ref(ipv6mr).cast(), size_of_val(ipv6mr))
                };
                (libc::IPPROTO_IPV6, libc::IPV6_DROP_MEMBERSHIP, bytes)
            },
            _ => (0, 0, &[]),
        }
    }
}

enum McastIface {
    V4(libc::ip_mreqn),
    V6(libc::c_int)
}

pub struct McastOutboundIf(McastIface);

impl McastOutboundIf {
    pub fn v4(iface: &Iface) -> Self {
        Self(McastIface::V4(libc::ip_mreqn {
            imr_multiaddr: libc::in_addr { s_addr: 0 },
            imr_address: libc::in_addr { s_addr: 0 },
            imr_ifindex: iface.as_raw().cast_signed(),
        }))
    }

    pub fn v6(iface: &Iface) -> Self {
        Self(McastIface::V6(iface.as_raw().cast_signed()))
    }
}

impl<P> SockOpt<P> for McastOutboundIf
where
    P: Protocol<Type = IpProtocol>,
{
    fn key(pro: P) -> (libc::c_int, libc::c_int) {
        match P::Type::version(pro) {
            Ip::V4 => (libc::IPPROTO_IP, libc::IP_MULTICAST_IF),
            Ip::V6 => (libc::IPPROTO_IPV6, libc::IPV6_MULTICAST_IF),
        }
    }
}

impl<P> SetSockOpt<P> for McastOutboundIf
where
    P: Protocol<Type = IpProtocol>,
{
    fn data(&self, pro: P) -> (libc::c_int, libc::c_int, &[u8]) {
        match (P::Type::version(pro), &self.0) {
            (Ip::V4, McastIface::V4(imr)) => {
                let bytes = unsafe {
                    slice::from_raw_parts(ptr::from_ref(imr).cast(), size_of_val(imr))
                };
                (libc::IPPROTO_IP, libc::IP_MULTICAST_IF, bytes)
            },
            (Ip::V6, McastIface::V6(ifi)) => {
                let bytes = unsafe {
                    slice::from_raw_parts(ptr::from_ref(ifi).cast(), size_of_val(ifi))
                };
                (libc::IPPROTO_IPV6, libc::IPV6_MULTICAST_IF, bytes)
            },
            _ => (0, 0, &[]),
        }
    }
}
