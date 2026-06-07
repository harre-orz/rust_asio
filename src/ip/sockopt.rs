use crate::error::{OsError, Result};
use crate::iface::{IfaceAddrRef, IfaceIdx, Ifaces, iface_name};
use crate::ip::endpoint::Ip;
use crate::ip::{IpProtocol, Tcp};
use crate::sockaddr::SockLen;
use crate::socket_base::{GetSockOpt, Protocol, SetSockOpt, SockOpt};
use std::mem::MaybeUninit;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::num::NonZeroU8;
use std::{ptr, slice};

#[cfg(windows)]
use windows_sys::Win32::Networking::WinSock;

pub struct NoDelay(libc::c_int);

impl NoDelay {
    const KEY: SockOpt = SockOpt {
        #[cfg(unix)]
        level: libc::IPPROTO_TCP,
        #[cfg(unix)]
        name: libc::TCP_NODELAY,

        #[cfg(windows)]
        level: WinSock::IPPROTO_TCP,
        #[cfg(windows)]
        name: WinSock::TCP_NODELAY,
    };

    pub const ON: Self = Self::new(true);
    pub const OFF: Self = Self::new(false);

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
    fn data(&self, _: Tcp) -> (SockOpt, &[u8]) {
        (Self::KEY, self.as_bytes())
    }
}

impl GetSockOpt<Tcp> for NoDelay {
    fn init(_: Tcp) -> (SockOpt, impl Fn(MaybeUninit<Self>, SockLen) -> Self) {
        (Self::KEY, move |uninit, _| unsafe { uninit.assume_init() })
    }
}

pub struct V6Only(libc::c_int);

impl V6Only {
    const KEY: SockOpt = SockOpt {
        #[cfg(unix)]
        level: libc::IPPROTO_IPV6,
        #[cfg(unix)]
        name: libc::IPV6_V6ONLY,

        #[cfg(windows)]
        level: WinSock::IPPROTO_IPV6,
        #[cfg(windows)]
        name: WinSock::IPV6_V6ONLY,
    };

    pub const ON: Self = Self::new(true);
    pub const OFF: Self = Self::new(false);

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
    fn data(&self, _: P) -> (SockOpt, &[u8]) {
        (Self::KEY, self.as_bytes())
    }
}

impl<P> GetSockOpt<P> for V6Only
where
    P: Protocol<Type = IpProtocol>,
{
    fn init(_: P) -> (SockOpt, impl Fn(MaybeUninit<Self>, SockLen) -> Self) {
        (Self::KEY, move |uninit, _| unsafe { uninit.assume_init() })
    }
}

pub struct UcastHops(libc::c_int);

impl UcastHops {
    const KEY_V4: SockOpt = SockOpt {
        #[cfg(unix)]
        level: libc::IPPROTO_IP,
        #[cfg(unix)]
        name: libc::IP_TTL,

        #[cfg(windows)]
        level: WinSock::IPPROTO_IP,
        #[cfg(windows)]
        name: WinSock::IP_TTL,
    };

    const KEY_V6: SockOpt = SockOpt {
        #[cfg(unix)]
        level: libc::IPPROTO_IPV6,
        #[cfg(unix)]
        name: libc::IPV6_UNICAST_HOPS,

        #[cfg(windows)]
        level: WinSock::IPPROTO_IPV6,
        #[cfg(windows)]
        name: WinSock::IPV6_UNICAST_HOPS,
    };

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
    fn data(&self, pro: P) -> (SockOpt, &[u8]) {
        match P::Type::version(pro) {
            Ip::V4 => (Self::KEY_V4, self.as_bytes()),
            Ip::V6 => (Self::KEY_V6, self.as_bytes()),
        }
    }
}

impl<P> GetSockOpt<P> for UcastHops
where
    P: Protocol<Type = IpProtocol>,
{
    fn init(pro: P) -> (SockOpt, impl Fn(MaybeUninit<Self>, SockLen) -> Self) {
        let init: fn(MaybeUninit<Self>, _: SockLen) -> Self =
            |uninit, _| unsafe { uninit.assume_init() };
        match P::Type::version(pro) {
            Ip::V4 => (Self::KEY_V4, init),
            Ip::V6 => (Self::KEY_V6, init),
        }
    }
}

pub struct McastLoop(libc::c_int);

impl McastLoop {
    const KEY_V4: SockOpt = SockOpt {
        #[cfg(unix)]
        level: libc::IPPROTO_IP,
        #[cfg(unix)]
        name: libc::IP_MULTICAST_LOOP,

        #[cfg(windows)]
        level: WinSock::IPPROTO_IP,
        #[cfg(windows)]
        name: WinSock::IP_MULTICAST_LOOP,
    };

    const KEY_V6: SockOpt = SockOpt {
        #[cfg(unix)]
        level: libc::IPPROTO_IPV6,
        #[cfg(unix)]
        name: libc::IPV6_MULTICAST_LOOP,

        #[cfg(windows)]
        level: WinSock::IPPROTO_IPV6,
        #[cfg(windows)]
        name: WinSock::IPV6_MULTICAST_LOOP,
    };

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
    fn data(&self, pro: P) -> (SockOpt, &[u8]) {
        match P::Type::version(pro) {
            Ip::V4 => (Self::KEY_V4, self.as_bytes()),
            Ip::V6 => (Self::KEY_V6, self.as_bytes()),
        }
    }
}

impl<P> GetSockOpt<P> for McastLoop
where
    P: Protocol<Type = IpProtocol>,
{
    fn init(pro: P) -> (SockOpt, impl Fn(MaybeUninit<Self>, SockLen) -> Self) {
        let init: fn(MaybeUninit<Self>, _: SockLen) -> Self =
            |uninit, _| unsafe { uninit.assume_init() };
        match P::Type::version(pro) {
            Ip::V4 => (Self::KEY_V4, init),
            Ip::V6 => (Self::KEY_V6, init),
        }
    }
}

pub struct McastHops(libc::c_int);

impl McastHops {
    const KEY_V4: SockOpt = SockOpt {
        #[cfg(unix)]
        level: libc::IPPROTO_IP,
        #[cfg(unix)]
        name: libc::IP_MULTICAST_TTL,

        #[cfg(windows)]
        level: WinSock::IPPROTO_IP,
        #[cfg(windows)]
        name: WinSock::IP_MULTICAST_TTL,
    };

    const KEY_V6: SockOpt = SockOpt {
        #[cfg(unix)]
        level: libc::IPPROTO_IPV6,
        #[cfg(unix)]
        name: libc::IPV6_MULTICAST_HOPS,

        #[cfg(windows)]
        level: WinSock::IPPROTO_IPV6,
        #[cfg(windows)]
        name: WinSock::IPV6_MULTICAST_HOPS,
    };

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
    fn data(&self, pro: P) -> (SockOpt, &[u8]) {
        match P::Type::version(pro) {
            Ip::V4 => (Self::KEY_V4, self.as_bytes()),
            Ip::V6 => (Self::KEY_V6, self.as_bytes()),
        }
    }
}

impl<P> GetSockOpt<P> for McastHops
where
    P: Protocol<Type = IpProtocol>,
{
    fn init(pro: P) -> (SockOpt, impl Fn(MaybeUninit<Self>, SockLen) -> Self) {
        let init: fn(MaybeUninit<Self>, _: SockLen) -> Self =
            |uninit, _| unsafe { uninit.assume_init() };
        match P::Type::version(pro) {
            Ip::V4 => (Self::KEY_V4, init),
            Ip::V6 => (Self::KEY_V6, init),
        }
    }
}

pub struct McastOutboundIf {
    #[cfg(unix)]
    v4: libc::in_addr,
    #[cfg(windows)]
    v4: WinSock::IN_ADDR,

    v6: libc::c_uint,
}

impl McastOutboundIf {
    const KEY_V4: SockOpt = SockOpt {
        #[cfg(unix)]
        level: libc::IPPROTO_IP,
        #[cfg(unix)]
        name: libc::IP_MULTICAST_IF,

        #[cfg(windows)]
        level: WinSock::IPPROTO_IP,
        #[cfg(windows)]
        name: WinSock::IP_MULTICAST_IF,
    };

    const KEY_V6: SockOpt = SockOpt {
        #[cfg(unix)]
        level: libc::IPPROTO_IPV6,
        #[cfg(unix)]
        name: libc::IPV6_MULTICAST_IF,

        #[cfg(windows)]
        level: WinSock::IPPROTO_IPV6,
        #[cfg(windows)]
        name: WinSock::IPV6_MULTICAST_IF,
    };

    fn new_impl(v4: u32, v6: libc::c_uint) -> Self {
        #[cfg(unix)]
        let v4 = libc::in_addr { s_addr: v4 };
        #[cfg(windows)]
        let v4 = WinSock::IN_ADDR {
            S_un: WinSock::IN_ADDR_0 { S_addr: v4 },
        };
        Self { v4: v4, v6: v6 }
    }

    pub fn v4(addr: &Ipv4Addr) -> Result<Self> {
        let ifaces = Ifaces::new()?;
        let mut v4_name = String::new();
        let mut v6_idx = None;
        for iface in ifaces.iter() {
            match iface.addr() {
                IfaceAddrRef::V4(ipv4, _) if ipv4 == addr => v4_name = String::from(iface.name()),
                IfaceAddrRef::V6(_, _, idx) if iface.name() == v4_name => v6_idx = Some(idx),
                _ => {}
            }
        }
        if v4_name.is_empty() {
            return Err(OsError::NO_SUCH_DEVICE);
        }
        if v6_idx.is_none() {
            for iface in ifaces.iter() {
                match iface.addr() {
                    IfaceAddrRef::V6(_, _, idx) if iface.name() == v4_name => v6_idx = Some(idx),
                    _ => {}
                }
            }
        }
        Ok(Self::new_impl(
            addr.to_bits(),
            v6_idx.map_or(0, |idx| idx.as_raw()),
        ))
    }

    pub fn v6(idx: IfaceIdx) -> Result<Self> {
        let ifaces = Ifaces::new()?;
        let v6_name = iface_name(idx)?;
        let mut v4_bits = 0;
        let mut v6_idx = None;
        for iface in ifaces.iter() {
            match iface.addr() {
                IfaceAddrRef::V4(ipv4, _) if iface.name() == v6_name => v4_bits = ipv4.to_bits(),
                IfaceAddrRef::V6(_, _, idx_) if idx == idx_ => v6_idx = Some(idx),
                _ => {}
            }
        }
        if let Some(v6_idx) = v6_idx {
            Ok(Self::new_impl(v4_bits, v6_idx.as_raw()))
        } else {
            Err(OsError::NO_SUCH_DEVICE)
        }
    }

    pub unsafe fn new_unchecked(addr: &Ipv4Addr, idx: IfaceIdx) -> Self {
        Self::new_impl(addr.to_bits(), idx.as_raw())
    }
}

impl<P> SetSockOpt<P> for McastOutboundIf
where
    P: Protocol<Type = IpProtocol>,
{
    fn data(&self, pro: P) -> (SockOpt, &[u8]) {
        match P::Type::version(pro) {
            Ip::V4 => {
                let bytes = unsafe {
                    slice::from_raw_parts(ptr::from_ref(&self.v4).cast(), size_of_val(&self.v4))
                };
                (Self::KEY_V4, bytes)
            }
            Ip::V6 => {
                let bytes = unsafe {
                    slice::from_raw_parts(ptr::from_ref(&self.v6).cast(), size_of_val(&self.v6))
                };
                (Self::KEY_V6, bytes)
            }
        }
    }
}

struct Ipv4MReq(
    #[cfg(target_os = "linux")] libc::ip_mreqn,
    #[cfg(target_os = "macos")] libc::ip_mreq,
    #[cfg(windows)] WinSock::IP_MREQ,
);

impl Ipv4MReq {
    const ZERO: Self = {
        #[cfg(target_os = "linux")]
        let mreq = libc::ip_mreqn {
            imr_multiaddr: libc::in_addr { s_addr: 0 },
            imr_address: libc::in_addr { s_addr: 0 },
            imr_ifindex: 0,
        };
        #[cfg(target_os = "macos")]
        let mreq = libc::ip_mreq {
            imr_multiaddr: libc::in_addr { s_addr: 0 },
            imr_interface: libc::in_addr { s_addr: 0 },
        };
        #[cfg(windows)]
        let mreq = WinSock::IP_MREQ {
            imr_multiaddr: WinSock::IN_ADDR {
                S_un: WinSock::IN_ADDR_0 { S_addr: 0 },
            },
            imr_interface: WinSock::IN_ADDR {
                S_un: WinSock::IN_ADDR_0 { S_addr: 0 },
            },
        };
        Self(mreq)
    };

    const unsafe fn new_unchecked(mcast_addr: &Ipv4Addr) -> Self {
        #[cfg(target_os = "linux")]
        let mreq = libc::ip_mreqn {
            imr_multiaddr: libc::in_addr {
                s_addr: mcast_addr.to_bits(),
            },
            imr_address: libc::in_addr { s_addr: 0 },
            imr_ifindex: 0,
        };
        #[cfg(target_os = "macos")]
        let mreq = libc::ip_mreq {
            imr_multiaddr: libc::in_addr {
                s_addr: mcast_addr.to_bits(),
            },
            imr_interface: libc::in_addr { s_addr: 0 },
        };
        #[cfg(windows)]
        let mreq = WinSock::IP_MREQ {
            imr_multiaddr: WinSock::IN_ADDR {
                S_un: WinSock::IN_ADDR_0 {
                    S_addr: mcast_addr.to_bits(),
                },
            },
            imr_interface: WinSock::IN_ADDR {
                S_un: WinSock::IN_ADDR_0 { S_addr: 0 },
            },
        };
        Self(mreq)
    }
}

struct Ipv6MReq(
    #[cfg(unix)] libc::ipv6_mreq,
    #[cfg(windows)] WinSock::IPV6_MREQ,
);

impl Ipv6MReq {
    const ZERO: Self = {
        #[cfg(unix)]
        let mreq = libc::ipv6_mreq {
            ipv6mr_multiaddr: libc::in6_addr { s6_addr: [0; 16] },
            ipv6mr_interface: 0,
        };
        #[cfg(windows)]
        let mreq = WinSock::IPV6_MREQ {
            ipv6mr_multiaddr: WinSock::IN6_ADDR {
                u: WinSock::IN6_ADDR_0 { Byte: [0; 16] },
            },
            ipv6mr_interface: 0,
        };
        Self(mreq)
    };

    const unsafe fn new_unchecked(mcast_addr: &Ipv6Addr) -> Self {
        #[cfg(unix)]
        let mreq = libc::ipv6_mreq {
            ipv6mr_multiaddr: libc::in6_addr {
                s6_addr: mcast_addr.octets(),
            },
            ipv6mr_interface: 0,
        };
        #[cfg(windows)]
        let mreq = WinSock::IPV6_MREQ {
            ipv6mr_multiaddr: WinSock::IN6_ADDR {
                u: WinSock::IN6_ADDR_0 {
                    Byte: mcast_addr.octets(),
                },
            },
            ipv6mr_interface: 0,
        };
        Self(mreq)
    }
}

pub struct McastJoinGroup {
    v4: Ipv4MReq,
    v6: Ipv6MReq,
}

impl McastJoinGroup {
    const KEY_V4: SockOpt = SockOpt {
        #[cfg(unix)]
        level: libc::IPPROTO_IP,
        #[cfg(unix)]
        name: libc::IP_ADD_MEMBERSHIP,

        #[cfg(windows)]
        level: WinSock::IPPROTO_IP,
        #[cfg(windows)]
        name: WinSock::IP_ADD_MEMBERSHIP,
    };

    const KEY_V6: SockOpt = SockOpt {
        #[cfg(unix)]
        level: libc::IPPROTO_IPV6,
        #[cfg(target_os = "linux")]
        name: libc::IPV6_ADD_MEMBERSHIP,
        #[cfg(target_os = "macos")]
        name: libc::IPV6_JOIN_GROUP,

        #[cfg(windows)]
        level: WinSock::IPPROTO_IP,
        #[cfg(windows)]
        name: WinSock::IPV6_ADD_MEMBERSHIP,
    };

    pub fn new(mcast_addr: &IpAddr) -> Result<Self> {
        match mcast_addr {
            IpAddr::V4(mcast_addr) => Self::v4(mcast_addr),
            IpAddr::V6(mcast_addr) => Self::v6(mcast_addr),
        }
    }

    pub const fn v4(mcast_addr: &Ipv4Addr) -> Result<Self> {
        if mcast_addr.is_multicast() {
            Ok(Self {
                v4: unsafe { Ipv4MReq::new_unchecked(mcast_addr) },
                v6: Ipv6MReq::ZERO,
            })
        } else {
            Err(OsError::INVALID_ARGUMENT)
        }
    }

    pub fn v6(mcast_addr: &Ipv6Addr) -> Result<Self> {
        if mcast_addr.is_multicast() {
            Ok(Self {
                v4: Ipv4MReq::ZERO,
                v6: unsafe { Ipv6MReq::new_unchecked(mcast_addr) },
            })
        } else {
            Err(OsError::INVALID_ARGUMENT)
        }
    }
}

impl<P> SetSockOpt<P> for McastJoinGroup
where
    P: Protocol<Type = IpProtocol>,
{
    fn data(&self, pro: P) -> (SockOpt, &[u8]) {
        match P::Type::version(pro) {
            Ip::V4 => {
                let bytes = unsafe {
                    slice::from_raw_parts(ptr::from_ref(&self.v4.0).cast(), size_of_val(&self.v4.0))
                };
                (Self::KEY_V4, bytes)
            }
            Ip::V6 => {
                let bytes = unsafe {
                    slice::from_raw_parts(ptr::from_ref(&self.v6.0).cast(), size_of_val(&self.v6.0))
                };
                (Self::KEY_V6, bytes)
            }
        }
    }
}

pub struct McastLeaveGroup {
    v4: Ipv4MReq,
    v6: Ipv6MReq,
}

impl McastLeaveGroup {
    const KEY_V4: SockOpt = SockOpt {
        #[cfg(unix)]
        level: libc::IPPROTO_IP,
        #[cfg(unix)]
        name: libc::IP_DROP_MEMBERSHIP,

        #[cfg(windows)]
        level: WinSock::IPPROTO_IP,
        #[cfg(windows)]
        name: WinSock::IP_DROP_MEMBERSHIP,
    };

    const KEY_V6: SockOpt = SockOpt {
        #[cfg(unix)]
        level: libc::IPPROTO_IPV6,
        #[cfg(target_os = "linux")]
        name: libc::IPV6_DROP_MEMBERSHIP,
        #[cfg(target_os = "macos")]
        name: libc::IPV6_LEAVE_GROUP,

        #[cfg(windows)]
        level: WinSock::IPPROTO_IPV6,
        #[cfg(windows)]
        name: WinSock::IPV6_DROP_MEMBERSHIP,
    };

    pub fn new(mcast_addr: &IpAddr) -> Result<Self> {
        match mcast_addr {
            IpAddr::V4(mcast_addr) => Self::v4(mcast_addr),
            IpAddr::V6(mcast_addr) => Self::v6(mcast_addr),
        }
    }

    pub fn v4(mcast_addr: &Ipv4Addr) -> Result<Self> {
        if mcast_addr.is_multicast() {
            Ok(Self {
                v4: unsafe { Ipv4MReq::new_unchecked(mcast_addr) },
                v6: Ipv6MReq::ZERO,
            })
        } else {
            Err(OsError::INVALID_ARGUMENT)
        }
    }

    pub fn v6(mcast_addr: &Ipv6Addr) -> Result<Self> {
        if mcast_addr.is_multicast() {
            Ok(Self {
                v4: Ipv4MReq::ZERO,
                v6: unsafe { Ipv6MReq::new_unchecked(mcast_addr) },
            })
        } else {
            Err(OsError::INVALID_ARGUMENT)
        }
    }
}

impl<P> SetSockOpt<P> for McastLeaveGroup
where
    P: Protocol<Type = IpProtocol>,
{
    fn data(&self, pro: P) -> (SockOpt, &[u8]) {
        match P::Type::version(pro) {
            Ip::V4 => {
                let bytes = unsafe {
                    slice::from_raw_parts(ptr::from_ref(&self.v4.0).cast(), size_of_val(&self.v4.0))
                };
                (Self::KEY_V4, bytes)
            }
            Ip::V6 => {
                let bytes = unsafe {
                    slice::from_raw_parts(ptr::from_ref(&self.v6.0).cast(), size_of_val(&self.v6.0))
                };
                (Self::KEY_V6, bytes)
            }
        }
    }
}

impl From<McastLeaveGroup> for McastJoinGroup {
    fn from(group: McastLeaveGroup) -> Self {
        Self {
            v4: group.v4,
            v6: group.v6,
        }
    }
}

impl From<McastJoinGroup> for McastLeaveGroup {
    fn from(group: McastJoinGroup) -> Self {
        Self {
            v4: group.v4,
            v6: group.v6,
        }
    }
}
