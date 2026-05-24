use crate::iface::Iface;
use crate::ip::endpoint::Ip;
use crate::ip::{IpProtocol, Tcp};
use crate::sockaddr::SockLen;
use crate::socket_base::{GetSockOpt, Protocol, SetSockOpt};
use std::mem::MaybeUninit;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::num::NonZeroU8;
use std::{ptr, slice};
#[cfg(windows)]
use windows_sys::Win32::Networking::WinSock;

pub struct NoDelay(libc::c_int);

impl NoDelay {
    #[cfg(unix)]
    const LEVEL: libc::c_int = libc::IPPROTO_TCP;

    #[cfg(windows)]
    const LEVEL: libc::c_int = WinSock::IPPROTO_TCP;

    #[cfg(unix)]
    const NAME: libc::c_int = libc::TCP_NODELAY;

    #[cfg(windows)]
    const NAME: libc::c_int = WinSock::TCP_NODELAY;

    pub const ON: Self = Self(1);
    pub const OFF: Self = Self(0);

    pub const fn new(on: bool) -> Self {
        Self(on as i32)
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }

    const fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(self).cast(), size_of_val(self)) }
    }
}

impl SetSockOpt<Tcp> for NoDelay {
    fn data(&self, _: Tcp) -> (libc::c_int, libc::c_int, &[u8]) {
        (Self::LEVEL, Self::NAME, self.as_bytes())
    }
}

impl GetSockOpt<Tcp> for NoDelay {
    fn init(
        _: Tcp,
    ) -> (
        libc::c_int,
        libc::c_int,
        impl Fn(MaybeUninit<Self>, SockLen) -> Self,
    ) {
        (Self::LEVEL, Self::NAME, move |uninit, _| unsafe {
            uninit.assume_init()
        })
    }
}

pub struct V6Only(libc::c_int);

impl V6Only {
    #[cfg(unix)]
    const LEVEL: libc::c_int = libc::IPPROTO_IPV6;

    #[cfg(windows)]
    const LEVEL: libc::c_int = WinSock::IPPROTO_IPV6;

    #[cfg(unix)]
    const NAME: libc::c_int = libc::IPV6_V6ONLY;

    #[cfg(windows)]
    const NAME: libc::c_int = WinSock::IPV6_V6ONLY;

    pub const ON: Self = Self(1);
    pub const OFF: Self = Self(0);

    pub const fn new(on: bool) -> Self {
        Self(on as i32)
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }

    const fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(self).cast(), size_of_val(self)) }
    }
}

impl<P> SetSockOpt<P> for V6Only
where
    P: Protocol<Type = IpProtocol>,
{
    fn data(&self, _: P) -> (libc::c_int, libc::c_int, &[u8]) {
        (Self::LEVEL, Self::NAME, self.as_bytes())
    }
}

impl<P> GetSockOpt<P> for V6Only
where
    P: Protocol<Type = IpProtocol>,
{
    fn init(
        _: P,
    ) -> (
        libc::c_int,
        libc::c_int,
        impl Fn(MaybeUninit<Self>, SockLen) -> Self,
    ) {
        (Self::LEVEL, Self::NAME, move |uninit, _| unsafe {
            uninit.assume_init()
        })
    }
}

pub struct UcastHops(libc::c_int);

impl UcastHops {
    #[cfg(unix)]
    const LEVEL: (libc::c_int, libc::c_int) = (libc::IPPROTO_IP, libc::IPPROTO_IPV6);

    #[cfg(windows)]
    const LEVEL: (libc::c_int, libc::c_int) = (WinSock::IPPROTO_IP, WinSock::IPPROTO_IPV6);

    #[cfg(unix)]
    const NAME: (libc::c_int, libc::c_int) = (libc::IP_TTL, libc::IPV6_UNICAST_HOPS);

    #[cfg(windows)]
    const NAME: (libc::c_int, libc::c_int) = (WinSock::IP_TTL, WinSock::IPV6_UNICAST_HOPS);

    pub const fn new(hops: NonZeroU8) -> Self {
        Self(hops.get() as libc::c_int)
    }

    pub const fn get(&self) -> u8 {
        self.0 as u8
    }

    fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(self).cast(), size_of_val(self)) }
    }
}

impl<P> SetSockOpt<P> for UcastHops
where
    P: Protocol<Type = IpProtocol>,
{
    fn data(&self, pro: P) -> (libc::c_int, libc::c_int, &[u8]) {
        match P::Type::version(pro) {
            Ip::V4 => (Self::LEVEL.0, Self::NAME.0, self.as_bytes()),
            Ip::V6 => (Self::LEVEL.1, Self::NAME.1, self.as_bytes()),
        }
    }
}

impl<P> GetSockOpt<P> for UcastHops
where
    P: Protocol<Type = IpProtocol>,
{
    fn init(
        pro: P,
    ) -> (
        libc::c_int,
        libc::c_int,
        impl Fn(MaybeUninit<Self>, SockLen) -> Self,
    ) {
        let init: fn(MaybeUninit<Self>, _: SockLen) -> Self =
            |uninit, _| unsafe { uninit.assume_init() };
        match P::Type::version(pro) {
            Ip::V4 => (Self::LEVEL.0, Self::NAME.0, init),
            Ip::V6 => (Self::LEVEL.1, Self::NAME.1, init),
        }
    }
}

pub struct McastLoop(libc::c_int);

impl McastLoop {
    #[cfg(unix)]
    const LEVEL: (libc::c_int, libc::c_int) = (libc::IPPROTO_IP, libc::IPPROTO_IPV6);

    #[cfg(windows)]
    const LEVEL: (libc::c_int, libc::c_int) = (WinSock::IPPROTO_IP, WinSock::IPPROTO_IPV6);

    #[cfg(unix)]
    const NAME: (libc::c_int, libc::c_int) = (libc::IP_MULTICAST_LOOP, libc::IPV6_MULTICAST_LOOP);

    #[cfg(windows)]
    const NAME: (libc::c_int, libc::c_int) =
        (WinSock::IP_MULTICAST_LOOP, WinSock::IPV6_MULTICAST_LOOP);

    pub const ON: Self = Self::new(true);
    pub const OFF: Self = Self::new(false);

    pub const fn new(on: bool) -> Self {
        Self(on as libc::c_int)
    }

    pub const fn get(&self) -> bool {
        self.0 != 0
    }

    fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(self).cast(), size_of_val(self)) }
    }
}

impl<P> SetSockOpt<P> for McastLoop
where
    P: Protocol<Type = IpProtocol>,
{
    fn data(&self, pro: P) -> (libc::c_int, libc::c_int, &[u8]) {
        match P::Type::version(pro) {
            Ip::V4 => (Self::LEVEL.0, Self::NAME.0, self.as_bytes()),
            Ip::V6 => (Self::LEVEL.1, Self::NAME.1, self.as_bytes()),
        }
    }
}

impl<P> GetSockOpt<P> for McastLoop
where
    P: Protocol<Type = IpProtocol>,
{
    fn init(
        pro: P,
    ) -> (
        libc::c_int,
        libc::c_int,
        impl Fn(MaybeUninit<Self>, SockLen) -> Self,
    ) {
        let init: fn(MaybeUninit<Self>, _: SockLen) -> Self =
            |uninit, _| unsafe { uninit.assume_init() };
        match P::Type::version(pro) {
            Ip::V4 => (Self::LEVEL.0, Self::NAME.0, init),
            Ip::V6 => (Self::LEVEL.1, Self::NAME.1, init),
        }
    }
}

pub struct McastHops(libc::c_int);

impl McastHops {
    #[cfg(unix)]
    const LEVEL: (libc::c_int, libc::c_int) = (libc::IPPROTO_IP, libc::IPPROTO_IPV6);

    #[cfg(windows)]
    const LEVEL: (libc::c_int, libc::c_int) = (WinSock::IPPROTO_IP, WinSock::IPPROTO_IPV6);

    #[cfg(unix)]
    const NAME: (libc::c_int, libc::c_int) = (libc::IP_MULTICAST_TTL, libc::IPV6_MULTICAST_HOPS);

    #[cfg(windows)]
    const NAME: (libc::c_int, libc::c_int) =
        (WinSock::IP_MULTICAST_TTL, WinSock::IPV6_MULTICAST_HOPS);

    pub const fn new(hops: NonZeroU8) -> Self {
        Self(hops.get() as libc::c_int)
    }

    pub const fn get(&self) -> u8 {
        self.0 as u8
    }

    const fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(self).cast(), size_of_val(self)) }
    }
}

impl<P> SetSockOpt<P> for McastHops
where
    P: Protocol<Type = IpProtocol>,
{
    fn data(&self, pro: P) -> (libc::c_int, libc::c_int, &[u8]) {
        match P::Type::version(pro) {
            Ip::V4 => (Self::LEVEL.0, Self::NAME.0, self.as_bytes()),
            Ip::V6 => (Self::LEVEL.1, Self::NAME.1, self.as_bytes()),
        }
    }
}

impl<P> GetSockOpt<P> for McastHops
where
    P: Protocol<Type = IpProtocol>,
{
    fn init(
        pro: P,
    ) -> (
        libc::c_int,
        libc::c_int,
        impl Fn(MaybeUninit<Self>, SockLen) -> Self,
    ) {
        let init: fn(MaybeUninit<Self>, _: SockLen) -> Self =
            |uninit, _| unsafe { uninit.assume_init() };
        match P::Type::version(pro) {
            Ip::V4 => (Self::LEVEL.0, Self::NAME.0, init),
            Ip::V6 => (Self::LEVEL.1, Self::NAME.1, init),
        }
    }
}

enum McastReq {
    #[cfg(target_os = "linux")]
    V4(libc::ip_mreqn),
    #[cfg(target_os = "macos")]
    V4(libc::ip_mreq),
    #[cfg(windows)]
    V4(WinSock::IP_MREQ),
    #[cfg(unix)]
    V6(libc::ipv6_mreq),
    #[cfg(windows)]
    V6(WinSock::IPV6_MREQ),
}

impl McastReq {
    #[cfg(target_os = "linux")]
    const fn v4(multiaddr: &Ipv4Addr) -> Self {
        Self::V4(libc::ip_mreqn {
            imr_multiaddr: libc::in_addr {
                s_addr: multiaddr.to_bits(),
            },
            imr_address: libc::in_addr { s_addr: 0 },
            imr_ifindex: 0,
        })
    }

    #[cfg(target_os = "macos")]
    const fn v4(multiaddr: &Ipv4Addr) -> Self {
        Self::V4(libc::ip_mreq {
            imr_multiaddr: libc::in_addr {
                s_addr: multiaddr.to_bits(),
            },
            imr_interface: libc::in_addr { s_addr: 0 },
        })
    }

    #[cfg(windows)]
    fn v4(multiaddr: &Ipv4Addr) -> Self {
        Self::V4(WinSock::IP_MREQ {
            imr_multiaddr: WinSock::IN_ADDR {
                S_un: WinSock::IN_ADDR_0 {
                    S_addr: multiaddr.to_bits(),
                },
            },
            imr_interface: WinSock::IN_ADDR {
                S_un: WinSock::IN_ADDR_0 { S_addr: 0 },
            },
        })
    }

    #[cfg(unix)]
    const fn v6(multiaddr: &Ipv6Addr) -> Self {
        Self::V6(libc::ipv6_mreq {
            ipv6mr_multiaddr: libc::in6_addr {
                s6_addr: multiaddr.octets(),
            },
            ipv6mr_interface: 0,
        })
    }

    #[cfg(windows)]
    fn v6(multiaddr: &Ipv6Addr) -> Self {
        Self::V6(WinSock::IPV6_MREQ {
            ipv6mr_multiaddr: WinSock::IN6_ADDR {
                u: WinSock::IN6_ADDR_0 {
                    Byte: multiaddr.octets(),
                },
            },
            ipv6mr_interface: 0,
        })
    }
}

pub struct McastJoinGroup(McastReq);

impl McastJoinGroup {
    #[cfg(unix)]
    const LEVEL: (libc::c_int, libc::c_int) = (libc::IPPROTO_IP, libc::IPPROTO_IPV6);

    #[cfg(windows)]
    const LEVEL: (libc::c_int, libc::c_int) = (WinSock::IPPROTO_IP, WinSock::IPPROTO_IPV6);

    #[cfg(target_os = "linux")]
    const NAME: (libc::c_int, libc::c_int) = (libc::IP_ADD_MEMBERSHIP, libc::IPV6_ADD_MEMBERSHIP);

    #[cfg(target_os = "macos")]
    const NAME: (libc::c_int, libc::c_int) = (libc::IP_ADD_MEMBERSHIP, libc::IPV6_JOIN_GROUP);

    #[cfg(windows)]
    const NAME: (libc::c_int, libc::c_int) =
        (WinSock::IP_ADD_MEMBERSHIP, WinSock::IPV6_ADD_MEMBERSHIP);

    pub fn new(mcast_addr: &IpAddr) -> Self {
        match mcast_addr {
            IpAddr::V4(mcast_addr) => Self::v4(mcast_addr),
            IpAddr::V6(mcast_addr) => Self::v6(mcast_addr),
        }
    }

    pub fn v4(mcast_addr: &Ipv4Addr) -> Self {
        Self(McastReq::v4(mcast_addr))
    }

    pub fn v6(mcast_addr: &Ipv6Addr) -> Self {
        Self(McastReq::v6(mcast_addr))
    }

    pub const fn into_leave_group(self) -> McastJoinGroup {
        McastJoinGroup(self.0)
    }
}

impl<P> SetSockOpt<P> for McastJoinGroup
where
    P: Protocol<Type = IpProtocol>,
{
    fn data(&self, pro: P) -> (libc::c_int, libc::c_int, &[u8]) {
        match (P::Type::version(pro), &self.0) {
            (Ip::V4, McastReq::V4(imr)) => {
                let bytes =
                    unsafe { slice::from_raw_parts(ptr::from_ref(imr).cast(), size_of_val(imr)) };
                (Self::LEVEL.0, Self::NAME.0, bytes)
            }
            (Ip::V6, McastReq::V6(ipv6mr)) => {
                let bytes = unsafe {
                    slice::from_raw_parts(ptr::from_ref(ipv6mr).cast(), size_of_val(ipv6mr))
                };
                (Self::LEVEL.1, Self::NAME.1, bytes)
            }
            _ => (0, 0, &[]),
        }
    }
}

pub struct McastLeaveGroup(McastReq);

impl McastLeaveGroup {
    #[cfg(unix)]
    const LEVEL: (libc::c_int, libc::c_int) = (libc::IPPROTO_IP, libc::IPPROTO_IPV6);

    #[cfg(windows)]
    const LEVEL: (libc::c_int, libc::c_int) = (WinSock::IPPROTO_IP, WinSock::IPPROTO_IPV6);

    #[cfg(target_os = "linux")]
    const NAME: (libc::c_int, libc::c_int) = (libc::IP_DROP_MEMBERSHIP, libc::IPV6_DROP_MEMBERSHIP);

    #[cfg(target_os = "macos")]
    const NAME: (libc::c_int, libc::c_int) = (libc::IP_DROP_MEMBERSHIP, libc::IPV6_LEAVE_GROUP);

    #[cfg(windows)]
    const NAME: (libc::c_int, libc::c_int) =
        (WinSock::IP_DROP_MEMBERSHIP, WinSock::IPV6_DROP_MEMBERSHIP);

    pub fn new(mcast_addr: &IpAddr) -> Self {
        match mcast_addr {
            IpAddr::V4(mcast_addr) => Self::v4(mcast_addr),
            IpAddr::V6(mcast_addr) => Self::v6(mcast_addr),
        }
    }

    pub fn v4(mcast_addr: &Ipv4Addr) -> Self {
        Self(McastReq::v4(mcast_addr))
    }

    pub fn v6(mcast_addr: &Ipv6Addr) -> Self {
        Self(McastReq::v6(mcast_addr))
    }

    pub const fn into_join_group(self) -> McastJoinGroup {
        McastJoinGroup(self.0)
    }
}

impl<P> SetSockOpt<P> for McastLeaveGroup
where
    P: Protocol<Type = IpProtocol>,
{
    fn data(&self, pro: P) -> (libc::c_int, libc::c_int, &[u8]) {
        match (P::Type::version(pro), &self.0) {
            (Ip::V4, McastReq::V4(imr)) => {
                let bytes =
                    unsafe { slice::from_raw_parts(ptr::from_ref(imr).cast(), size_of_val(imr)) };
                (Self::LEVEL.0, Self::NAME.0, bytes)
            }
            (Ip::V6, McastReq::V6(ipv6mr)) => {
                let bytes = unsafe {
                    slice::from_raw_parts(ptr::from_ref(ipv6mr).cast(), size_of_val(ipv6mr))
                };
                (Self::LEVEL.1, Self::NAME.1, bytes)
            }
            _ => (0, 0, &[]),
        }
    }
}

enum McastIface {
    #[cfg(target_os = "linux")]
    V4(libc::ip_mreqn),
    #[cfg(target_os = "macos")]
    V4(libc::in_addr),
    #[cfg(windows)]
    V4(WinSock::IN_ADDR),
    V6(libc::c_int),
}

impl McastIface {
    #[cfg(target_os = "linux")]
    const fn v4(multiaddr: &Ipv4Addr) -> Self {
        McastIface::V4(libc::ip_mreqn {
            imr_multiaddr: libc::in_addr {
                s_addr: multiaddr.to_bits(),
            },
            imr_address: libc::in_addr { s_addr: 0 },
            imr_ifindex: 0,
        })
    }

    #[cfg(target_os = "macos")]
    const fn v4(multiaddr: &Ipv4Addr) -> Self {
        McastIface::V4(libc::in_addr {
            s_addr: multiaddr.to_bits(),
        })
    }

    #[cfg(windows)]
    const fn v4(multiaddr: &Ipv4Addr) -> Self {
        McastIface::V4(WinSock::IN_ADDR {
            S_un: WinSock::IN_ADDR_0 {
                S_addr: multiaddr.to_bits(),
            },
        })
    }
}

pub struct McastOutboundIf(McastIface);

impl McastOutboundIf {
    #[cfg(unix)]
    const LEVEL: (libc::c_int, libc::c_int) = (libc::IPPROTO_IP, libc::IPPROTO_IPV6);

    #[cfg(windows)]
    const LEVEL: (libc::c_int, libc::c_int) = (WinSock::IPPROTO_IP, WinSock::IPPROTO_IPV6);

    #[cfg(unix)]
    const NAME: (libc::c_int, libc::c_int) = (libc::IP_MULTICAST_IF, libc::IPV6_MULTICAST_IF);

    #[cfg(windows)]
    const NAME: (libc::c_int, libc::c_int) = (WinSock::IP_MULTICAST_IF, WinSock::IPV6_MULTICAST_IF);

    pub fn v4(multiaddr: &Ipv4Addr) -> Self {
        Self(McastIface::v4(multiaddr))
    }

    pub fn v6(iface: &Iface) -> Self {
        Self(McastIface::V6(iface.as_raw().cast_signed()))
    }
}

impl<P> SetSockOpt<P> for McastOutboundIf
where
    P: Protocol<Type = IpProtocol>,
{
    fn data(&self, pro: P) -> (libc::c_int, libc::c_int, &[u8]) {
        match (P::Type::version(pro), &self.0) {
            (Ip::V4, McastIface::V4(imr)) => {
                let bytes =
                    unsafe { slice::from_raw_parts(ptr::from_ref(imr).cast(), size_of_val(imr)) };
                (Self::LEVEL.0, Self::NAME.0, bytes)
            }
            (Ip::V6, McastIface::V6(ifi)) => {
                let bytes =
                    unsafe { slice::from_raw_parts(ptr::from_ref(ifi).cast(), size_of_val(ifi)) };
                (Self::LEVEL.1, Self::NAME.1, bytes)
            }
            _ => (0, 0, &[]),
        }
    }
}
