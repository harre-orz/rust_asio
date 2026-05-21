use crate::ip::endpoint::Ip;
use crate::ip::{IpProtocol, Tcp};
use crate::socket_base::{GetSockOpt, Protocol, SetSockOpt};
use std::mem::MaybeUninit;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::num::NonZeroU8;
use std::{ptr, slice};

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

pub struct UcastHops(libc::c_int);

impl UcastHops {
    pub const fn new(count: NonZeroU8) -> Self {
        Self(count.get() as libc::c_int)
    }

    pub const fn get(&self) -> u8 {
        self.0 as u8
    }
}

enum McastMember {
    V4(libc::ip_mreq),
    V6(libc::ipv6_mreq),
}

pub struct McastJoin(McastMember);

impl McastJoin {
    pub fn v4(multiaddr: Ipv4Addr) -> Self {
        Self(McastMember::V4(libc::ip_mreq {
            imr_multiaddr: libc::in_addr {
                s_addr: multiaddr.to_bits(),
            },
            imr_interface: libc::in_addr { s_addr: 0 },
        }))
    }

    pub fn v6(multiaddr: Ipv6Addr) -> Self {
        Self(McastMember::V6(libc::ipv6_mreq {
            ipv6mr_multiaddr: libc::in6_addr {
                s6_addr: multiaddr.octets(),
            },
            ipv6mr_interface: 0,
        }))
    }
}

pub struct McastLeave(McastMember);

impl McastLeave {
    pub fn v4(multiaddr: Ipv4Addr) -> Self {
        Self(McastMember::V4(libc::ip_mreq {
            imr_multiaddr: libc::in_addr {
                s_addr: multiaddr.to_bits(),
            },
            imr_interface: libc::in_addr { s_addr: 0 },
        }))
    }

    pub fn v6(multiaddr: Ipv6Addr) -> Self {
        Self(McastMember::V6(libc::ipv6_mreq {
            ipv6mr_multiaddr: libc::in6_addr {
                s6_addr: multiaddr.octets(),
            },
            ipv6mr_interface: 0,
        }))
    }
}

const fn init<T>(uninit: MaybeUninit<T>, _: usize) -> T {
    unsafe { uninit.assume_init() }
}

const fn as_bytes<T>(data: &T) -> &[u8] {
    unsafe { slice::from_raw_parts(ptr::from_ref(data).cast(), size_of::<T>()) }
}

#[cfg(unix)]
mod ffi {
    use super::*;

    impl SetSockOpt<Tcp> for NoDelay {
        fn data(&self, _: Tcp) -> (libc::c_int, libc::c_int, &[u8]) {
            (libc::IPPROTO_TCP, libc::TCP_NODELAY, as_bytes(&self.0))
        }
    }

    impl GetSockOpt<Tcp> for NoDelay {
        fn init(_: Tcp) -> (libc::c_int, libc::c_int, impl Fn(MaybeUninit<Self>, usize) -> Self) {
            (libc::IPPROTO_TCP, libc::TCP_NODELAY, init)
        }
    }

    impl<P> SetSockOpt<P> for V6Only
    where
        P: Protocol<Type = IpProtocol>,
    {
        fn data(&self, _: P) -> (libc::c_int, libc::c_int, &[u8]) {
            (libc::IPPROTO_IPV6, libc::IPV6_V6ONLY, as_bytes(&self.0))
        }
    }

    impl<P> GetSockOpt<P> for V6Only
    where
        P: Protocol<Type = IpProtocol>,
    {
        fn init(_: P) -> (libc::c_int, libc::c_int, impl Fn(MaybeUninit<Self>, usize) -> Self) {
            (libc::IPPROTO_IPV6, libc::IPV6_V6ONLY, init)
        }
    }

    impl<P> SetSockOpt<P> for UcastHops
    where
        P: Protocol<Type = IpProtocol>,
    {
        fn data(&self, pro: P) -> (libc::c_int, libc::c_int, &[u8]) {
            match IpProtocol::version(pro) {
                Ip::V4 => (libc::IPPROTO_IP, libc::IP_TTL, as_bytes(&self.0)),
                Ip::V6 => (
                    libc::IPPROTO_IPV6,
                    libc::IPV6_UNICAST_HOPS,
                    as_bytes(&self.0),
                ),
            }
        }
    }

    impl<P> GetSockOpt<P> for UcastHops
    where
        P: Protocol<Type = IpProtocol>,
    {
        fn init(pro: P) -> (libc::c_int, libc::c_int, impl Fn(MaybeUninit<Self>, usize) -> Self) {
            match IpProtocol::version(pro) {
                Ip::V4 => (libc::IPPROTO_IP, libc::IP_TTL, init),
                Ip::V6 => (libc::IPPROTO_IPV6, libc::IPV6_UNICAST_HOPS, init),
            }
        }
    }

    impl<P> SetSockOpt<P> for McastLoop
    where
        P: Protocol<Type = IpProtocol>,
    {
        fn data(&self, pro: P) -> (libc::c_int, libc::c_int, &[u8]) {
            match IpProtocol::version(pro) {
                Ip::V4 => (libc::IPPROTO_IP, libc::IP_MULTICAST_LOOP, as_bytes(&self.0)),
                Ip::V6 => (
                    libc::IPPROTO_IPV6,
                    libc::IPV6_MULTICAST_LOOP,
                    as_bytes(&self.0),
                ),
            }
        }
    }

    impl<P> GetSockOpt<P> for McastLoop
    where
        P: Protocol<Type = IpProtocol>,
    {
        fn init(pro: P) -> (libc::c_int, libc::c_int, impl Fn(MaybeUninit<Self>, usize) -> Self) {
            match IpProtocol::version(pro) {
                Ip::V4 => (libc::IPPROTO_IP, libc::IP_MULTICAST_LOOP, init),
                Ip::V6 => (libc::IPPROTO_IPV6, libc::IPV6_MULTICAST_LOOP, init),
            }
        }
    }

    impl<P> SetSockOpt<P> for McastJoin
    where
        P: Protocol<Type = IpProtocol>,
    {
        fn data(&self, pro: P) -> (libc::c_int, libc::c_int, &[u8]) {
            match (IpProtocol::version(pro), &self.0) {
                (Ip::V4, McastMember::V4(mreq)) => {
                    (libc::IPPROTO_IP, libc::IP_ADD_MEMBERSHIP, as_bytes(mreq))
                }
                #[cfg(target_os = "linux")]
                (Ip::V6, McastMember::V6(mreq)) => (
                    libc::IPPROTO_IPV6,
                    libc::IPV6_ADD_MEMBERSHIP,
                    as_bytes(mreq),
                ),
                #[cfg(target_os = "macos")]
                (Ip::V6, McastMember::V6(mreq)) => {
                    (libc::IPPROTO_IPV6, libc::IPV6_JOIN_GROUP, as_bytes(mreq))
                }
                _ => (0, 0, &[]),
            }
        }
    }

    impl<P> SetSockOpt<P> for McastLeave
    where
        P: Protocol<Type = IpProtocol>,
    {
        fn data(&self, pro: P) -> (libc::c_int, libc::c_int, &[u8]) {
            match (IpProtocol::version(pro), &self.0) {
                (Ip::V4, McastMember::V4(mreq)) => {
                    (libc::IPPROTO_IP, libc::IP_ADD_MEMBERSHIP, as_bytes(mreq))
                }
                #[cfg(target_os = "linux")]
                (Ip::V6, McastMember::V6(mreq)) => (
                    libc::IPPROTO_IPV6,
                    libc::IPV6_ADD_MEMBERSHIP,
                    as_bytes(mreq),
                ),
                #[cfg(target_os = "macos")]
                (Ip::V6, McastMember::V6(mreq)) => {
                    (libc::IPPROTO_IPV6, libc::IPV6_LEAVE_GROUP, as_bytes(mreq))
                }
                _ => (0, 0, &[]),
            }
        }
    }
}
